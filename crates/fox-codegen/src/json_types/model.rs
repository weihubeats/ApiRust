//! JSON → 类型树：递归推断 + 数组元素类型合并。

use serde_json::Value;

use super::naming::to_pascal_case;

/// 推断出的字段类型（中立模型，与目标语言无关）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FieldType {
    Str,
    Int,
    Float,
    Bool,
    /// null / 空数组 / 混合类型：无法确定 → 各语言动态类型兜底。
    Unknown,
    Array(Box<FieldType>),
    Record(RecordDef),
}

/// 结构体定义（`preferred` 为候选名，解析阶段再定稿去重）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordDef {
    pub(crate) preferred: String,
    pub(crate) fields: Vec<RecordField>,
}

/// 字段：保留原始 JSON key（用于改名/rename/tag），类型为推断结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordField {
    pub(crate) json_key: String,
    pub(crate) ty: FieldType,
}

/// 结构体候选名：PascalCase；空结果兜底 `Item`（如空 key）；纯分隔符等仍合法。
fn preferred_name(key: &str) -> String {
    let pascal = to_pascal_case(key);
    if pascal.is_empty() {
        "Item".to_string()
    } else {
        pascal
    }
}

/// 递归推断单个值（`key` 用于命名嵌套结构体）。
fn infer_value(key: &str, value: &Value) -> FieldType {
    match value {
        Value::Null => FieldType::Unknown,
        Value::Bool(_) => FieldType::Bool,
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                FieldType::Int
            } else {
                FieldType::Float
            }
        }
        Value::String(_) => FieldType::Str,
        Value::Array(items) => {
            // 逐元素推断并融合；空数组 → Unknown。
            let mut merged: Option<FieldType> = None;
            for item in items {
                let ty = infer_value(key, item);
                merged = Some(match merged {
                    None => ty,
                    Some(prev) => merge_types(prev, ty),
                });
            }
            FieldType::Array(Box::new(merged.unwrap_or(FieldType::Unknown)))
        }
        Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| RecordField {
                    json_key: k.clone(),
                    ty: infer_value(k, v),
                })
                .collect();
            FieldType::Record(RecordDef {
                preferred: preferred_name(key),
                fields,
            })
        }
    }
}

/// 融合两个类型（数组元素合并）。
///
/// 规则：Unknown（null/空）可被任意真实类型吸收；Int+Float → Float；
/// 相同内容的结构体复用；其余冲突 → Unknown。
fn merge_types(a: FieldType, b: FieldType) -> FieldType {
    use FieldType::*;
    match (a, b) {
        (Unknown, t) | (t, Unknown) => t,
        (Str, Str) => Str,
        (Int, Int) => Int,
        (Bool, Bool) => Bool,
        (Int, Float) | (Float, Int) | (Float, Float) => Float,
        (Array(x), Array(y)) => Array(Box::new(merge_types(*x, *y))),
        (Record(x), Record(y)) if same_fields(&x.fields, &y.fields) => Record(x),
        _ => Unknown,
    }
}

/// 类型树形状相等（忽略结构体候选名，只比字段 key 与类型形状）。
pub(crate) fn same_shape(a: &FieldType, b: &FieldType) -> bool {
    match (a, b) {
        (FieldType::Record(x), FieldType::Record(y)) => same_fields(&x.fields, &y.fields),
        (FieldType::Array(x), FieldType::Array(y)) => same_shape(x, y),
        (x, y) => x == y,
    }
}

fn same_fields(x: &[RecordField], y: &[RecordField]) -> bool {
    x.len() == y.len()
        && x.iter()
            .zip(y)
            .all(|(a, b)| a.json_key == b.json_key && same_shape(&a.ty, &b.ty))
}

/// 根节点推断：必须为 JSON 对象，或全部元素均为对象/省略 null 的数组。
pub(crate) fn infer_root(value: &Value, root_key: &str) -> Result<FieldType, String> {
    match value {
        Value::Object(_) => Ok(infer_value(root_key, value)),
        Value::Array(items) => {
            let mut merged: Option<FieldType> = None;
            for item in items {
                let ty = infer_value(root_key, item);
                merged = Some(match merged {
                    None => ty,
                    Some(prev) => merge_types(prev, ty),
                });
            }
            let Some(root) = merged else {
                return Err("根节点须为 JSON 对象，或元素均为对象的数组".to_string());
            };
            match root {
                FieldType::Record(_) => Ok(root),
                _ => Err("根节点须为 JSON 对象，或元素均为对象的数组".to_string()),
            }
        }
        _ => Err("根节点须为 JSON 对象，或元素均为对象的数组".to_string()),
    }
}
