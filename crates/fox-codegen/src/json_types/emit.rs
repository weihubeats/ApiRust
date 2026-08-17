//! 名称定稿（去重/冲突降级）与三种语言的类型定义渲染。

use std::collections::HashSet;

use crate::util::dq;

use super::model::{same_shape, FieldType, RecordDef, RecordField};
use super::naming::{
    go_field_ident, java_field_ident, rust_field_ident, to_camel_case, to_pascal_case,
    to_snake_case,
};

/// 解析完成的中立模型：`records` 为全部结构体（根结构体在首位，深度优先顺序）。
pub(crate) struct ResolvedModel {
    pub(crate) records: Vec<RecordDef>,
}

/// 名称登记处：同名同形复用、同名异形按祖先链限定。
struct NameRegistry {
    records: Vec<RecordDef>,
    names: HashSet<String>,
    /// 已完成字段解析的结构体名（占位符不参与形状复用比较）。
    completed: HashSet<String>,
}

impl NameRegistry {
    fn new() -> Self {
        NameRegistry {
            records: Vec::new(),
            names: HashSet::new(),
            completed: HashSet::new(),
        }
    }

    /// 解析一个结构体：先占位登记、再递归字段，保证父结构体先于子结构体入队。
    fn resolve_fields(&mut self, name: String, def: &RecordDef) {
        self.names.insert(name.clone());
        self.records.push(RecordDef {
            preferred: name.clone(),
            fields: Vec::new(),
        });
        let idx = self.records.len() - 1;
        let fields = def
            .fields
            .iter()
            .map(|field| RecordField {
                json_key: field.json_key.clone(),
                ty: self.resolve_ty(&field.ty, std::slice::from_ref(&name)),
            })
            .collect();
        self.records[idx].fields = fields;
        self.completed.insert(name);
    }

    /// 递归解析字段类型：嵌套 Record 需要认领名字。
    fn resolve_ty(&mut self, ty: &FieldType, ancestors: &[String]) -> FieldType {
        match ty {
            FieldType::Array(inner) => {
                FieldType::Array(Box::new(self.resolve_ty(inner, ancestors)))
            }
            FieldType::Record(def) => {
                let name = self.claim(def, ancestors);
                let mut chain = ancestors.to_vec();
                chain.push(name.clone());
                let fields = def
                    .fields
                    .iter()
                    .map(|field| RecordField {
                        json_key: field.json_key.clone(),
                        ty: self.resolve_ty(&field.ty, &chain),
                    })
                    .collect();
                FieldType::Record(RecordDef {
                    preferred: name,
                    fields,
                })
            }
            primitive => primitive.clone(),
        }
    }

    /// 认领嵌套结构体名字，冲突时按 祖先结构体名 + 候选名 逐级限定。
    fn claim(&mut self, def: &RecordDef, ancestors: &[String]) -> String {
        // 1) 同名同形 → 直接复用（跳过未完成的占位符）
        if let Some(existing) = self
            .records
            .iter()
            .filter(|record| self.completed.contains(&record.preferred))
            .find(|record| same_shape_records(record, def))
        {
            return existing.preferred.clone();
        }
        // 2) 候选名未被占用
        let candidate = def.preferred.clone();
        if !self.names.contains(&candidate) {
            self.resolve_fields(candidate.clone(), def);
            return candidate;
        }
        // 3) 祖先限定：{Parent}{Candidate}
        for ancestor in ancestors.iter().rev() {
            let qualified = format!("{ancestor}{candidate}");
            if !self.names.contains(&qualified) {
                self.resolve_fields(qualified.clone(), def);
                return qualified;
            }
        }
        // 4) 数字后缀兜底
        let mut suffix = 2;
        loop {
            let numbered = format!("{candidate}{suffix}");
            if !self.names.contains(&numbered) {
                self.resolve_fields(numbered.clone(), def);
                return numbered;
            }
            suffix += 1;
        }
    }
}

fn same_shape_records(a: &RecordDef, b: &RecordDef) -> bool {
    a.fields.len() == b.fields.len()
        && a.fields
            .iter()
            .zip(&b.fields)
            .all(|(x, y)| x.json_key == y.json_key && same_shape(&x.ty, &y.ty))
}

/// 解析整棵类型树：根结构体定稿名 = 候选名（用户传入 root_name）。
pub(crate) fn resolve(root: RecordDef) -> ResolvedModel {
    let mut registry = NameRegistry::new();
    let root_name = root.preferred.clone();
    registry.resolve_fields(root_name, &root);
    ResolvedModel {
        records: registry.records,
    }
}

// ---------- 类型映射 ----------

fn rust_ty(ty: &FieldType) -> String {
    match ty {
        FieldType::Str => "String".to_string(),
        FieldType::Int => "i64".to_string(),
        FieldType::Float => "f64".to_string(),
        FieldType::Bool => "bool".to_string(),
        FieldType::Unknown => "serde_json::Value".to_string(),
        FieldType::Array(inner) => format!("Vec<{}>", rust_ty(inner)),
        FieldType::Record(def) => def.preferred.clone(),
    }
}

fn java_ty(ty: &FieldType) -> String {
    match ty {
        FieldType::Str => "String".to_string(),
        FieldType::Int => "Long".to_string(),
        FieldType::Float => "Double".to_string(),
        FieldType::Bool => "Boolean".to_string(),
        FieldType::Unknown => "Object".to_string(),
        FieldType::Array(inner) => format!("List<{}>", java_ty(inner)),
        FieldType::Record(def) => def.preferred.clone(),
    }
}

fn go_ty(ty: &FieldType) -> String {
    match ty {
        FieldType::Str => "string".to_string(),
        FieldType::Int => "int64".to_string(),
        FieldType::Float => "float64".to_string(),
        FieldType::Bool => "bool".to_string(),
        FieldType::Unknown => "interface{}".to_string(),
        FieldType::Array(inner) => format!("[]{}", go_ty(inner)),
        FieldType::Record(def) => def.preferred.clone(),
    }
}

// ---------- 字段名 ----------

fn rust_field_name(key: &str) -> (String, bool) {
    let snake = to_snake_case(key);
    let ident = rust_field_ident(snake);
    // `r#` 是语法前缀，serde 序列化名仍是原词，仅防护后文字不同才需 rename。
    let wire = ident.trim_start_matches("r#").to_string();
    (ident, wire != key)
}

fn java_field_name(key: &str) -> (String, bool) {
    let camel = to_camel_case(key);
    let ident = java_field_ident(camel);
    // Jackson 以字段名为序列化名，`_class` 这类防护名必须显式 @JsonProperty。
    let needs_rename = ident != key;
    (ident, needs_rename)
}

fn go_field_name(key: &str) -> String {
    go_field_ident(to_pascal_case(key))
}

/// Go struct tag：默认反引号原始字符串；key 含反引号时退化为双引号字符串 tag。
fn go_tag(key: &str) -> String {
    if key.contains('`') {
        format!("\"json:\\\"{}\\\"\"", dq(key))
    } else {
        format!("`json:\"{}\"`", key)
    }
}

// ---------- 渲染 ----------

pub(crate) fn emit_rust(model: &ResolvedModel) -> String {
    let mut out = String::from("// 由 RustFox 自动生成，请勿手改。\n\n");
    for record in &model.records {
        out.push_str("#[derive(Debug, Serialize, Deserialize)]\n");
        out.push_str(&format!("pub struct {} {{\n", record.preferred));
        for field in &record.fields {
            let (ident, renamed) = rust_field_name(&field.json_key);
            if renamed {
                out.push_str(&format!(
                    "    #[serde(rename = \"{}\")]\n",
                    dq(&field.json_key)
                ));
            }
            out.push_str(&format!("    pub {ident}: {},\n", rust_ty(&field.ty)));
        }
        out.push_str("}\n\n");
    }
    out
}

pub(crate) fn emit_java(model: &ResolvedModel) -> String {
    let mut out = String::from("// 由 RustFox 自动生成，请勿手改。\n\n");
    out.push_str("import com.fasterxml.jackson.annotation.JsonProperty;\n");
    out.push_str("import java.util.List;\n");
    out.push_str("import lombok.Builder;\n");
    out.push_str("import lombok.Data;\n\n");
    for record in &model.records {
        out.push_str("@Data\n@Builder\n");
        out.push_str(&format!("class {} {{\n", record.preferred));
        for field in &record.fields {
            let (ident, renamed) = java_field_name(&field.json_key);
            if renamed {
                out.push_str(&format!("    @JsonProperty(\"{}\")\n", dq(&field.json_key)));
            }
            out.push_str(&format!("    private {} {};\n", java_ty(&field.ty), ident));
        }
        out.push_str("}\n\n");
    }
    out
}

pub(crate) fn emit_go(model: &ResolvedModel) -> String {
    let mut out = String::from("// 由 RustFox 自动生成，请勿手改。\n\n");
    for record in &model.records {
        out.push_str(&format!("type {} struct {{\n", record.preferred));
        for field in &record.fields {
            out.push_str(&format!(
                "    {} {} {}\n",
                go_field_name(&field.json_key),
                go_ty(&field.ty),
                go_tag(&field.json_key)
            ));
        }
        out.push_str("}\n\n");
    }
    out
}
