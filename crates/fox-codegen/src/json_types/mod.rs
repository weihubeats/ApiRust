//! JSON 自动转多语言类型/结构体。
//!
//! 输入：一段 JSON 样例（如 API 的 Request/Response body）；
//! 输出：目标语言的结构体/类定义。
//!
//! 流水线（三层解耦）：
//! 1. [`model`]：递归推断 → 中立类型树（String / i64 / f64 / bool / Vec / 嵌套 Record），
//!    数组元素类型自动融合（`[1]` → i64，`[1, 2.5]` → f64，null 被真实类型吸收）；
//! 2. [`emit::resolve`]：名称定稿——结构体名 PascalCase、同名同形复用、
//!    同名异形按祖先链限定（`Data` 冲突 → `InfoData`）；
//! 3. [`emit`]：按语言渲染：Rust 强制 snake_case + `#[serde(rename)]`；
//!    Go 字段 PascalCase + `json:"..."` tag（保留原始 key，保证线上格式不变）；
//!    Java 字段 camelCase + `@JsonProperty` + Lombok `@Data @Builder`。
//!
//! 类型映射：Integer → `i64`/`Long`/`int64`，Float → `f64`/`Double`/`float64`，
//! String → `String`/`String`/`string`，Array → `Vec<T>`/`List<T>`/`[]T`，
//! null / 混合类型 → `serde_json::Value`/`Object`/`interface{}`。

mod emit;
mod model;
mod naming;

use serde_json::Value;

use crate::error::CodeGenError;

use model::FieldType;

/// 目标语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeLang {
    Rust,
    Java,
    Go,
}

impl TypeLang {
    pub fn all() -> &'static [TypeLang] {
        &[TypeLang::Rust, TypeLang::Java, TypeLang::Go]
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "rust" => Some(TypeLang::Rust),
            "java" => Some(TypeLang::Java),
            "go" => Some(TypeLang::Go),
            _ => None,
        }
    }

    /// 展示名。
    pub fn label(&self) -> &'static str {
        match self {
            TypeLang::Rust => "Rust (serde)",
            TypeLang::Java => "Java (Lombok + Jackson)",
            TypeLang::Go => "Go (encoding/json)",
        }
    }

    /// 文件后缀。
    pub fn file_suffix(&self) -> &'static str {
        match self {
            TypeLang::Rust => "rs",
            TypeLang::Java => "java",
            TypeLang::Go => "go",
        }
    }
}

/// 从 JSON 样例生成目标语言类型定义。
///
/// `root_name` 为根结构体名（任意识别符风格均可，自动转 PascalCase）。
///
/// # Errors
/// - [`CodeGenError::JsonParse`]：JSON 语法错误
/// - [`CodeGenError::TypeInference`]：根节点非对象/对象数组
pub fn json_to_structs(
    json: &str,
    root_name: &str,
    lang: TypeLang,
) -> Result<String, CodeGenError> {
    let root_name = root_name.trim();
    if root_name.is_empty() {
        return Err(CodeGenError::TypeInference(
            "root_name 不能为空".to_string(),
        ));
    }
    let value: Value =
        serde_json::from_str(json).map_err(|err| CodeGenError::JsonParse(err.to_string()))?;
    let root = model::infer_root(&value, root_name).map_err(CodeGenError::TypeInference)?;
    let FieldType::Record(root_def) = root else {
        unreachable!("infer_root 已保证根节点为 Record");
    };
    let model = emit::resolve(root_def);
    Ok(match lang {
        TypeLang::Rust => emit::emit_rust(&model),
        TypeLang::Java => emit::emit_java(&model),
        TypeLang::Go => emit::emit_go(&model),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_snake_case_fields_with_rename() {
        let code = json_to_structs(
            r#"{"userName":"a","phoneNumber":"b","rawKey":"c"}"#,
            "userResponse",
            TypeLang::Rust,
        )
        .unwrap();
        assert!(code.contains("pub struct UserResponse {"));
        assert!(code.contains("pub user_name: String,"));
        assert!(code.contains("#[serde(rename = \"userName\")]"));
        // rawKey 非 snake_case，同样需要 rename 保线上格式。
        assert!(code.contains("#[serde(rename = \"rawKey\")]"));
    }

    #[test]
    fn rust_nested_object_and_array() {
        let code = json_to_structs(
            r#"{"user":{"id":1,"profile":{"bio":"hi"}},"items":[{"sku":"a"}],"tags":["x","y"]}"#,
            "Response",
            TypeLang::Rust,
        )
        .unwrap();
        assert!(code.contains("pub struct User {"));
        assert!(code.contains("pub id: i64,"));
        assert!(code.contains("pub profile: Profile,"));
        assert!(code.contains("pub struct Profile {"));
        assert!(code.contains("pub items: Vec<Items>,"));
        assert!(code.contains("pub tags: Vec<String>,"));
        assert!(code.contains("pub struct Items {"));
    }

    #[test]
    fn root_struct_emitted_first() {
        let code = json_to_structs(
            r#"{"user":{"id":1},"items":[{"sku":"a"}]}"#,
            "Response",
            TypeLang::Rust,
        )
        .unwrap();
        let root = code.find("pub struct Response").unwrap();
        let child = code.find("pub struct User").unwrap();
        let grand_child = code.find("pub struct Items").unwrap();
        // 根结构体必定最先输出；同层子结构体顺序由 JSON key 排序决定。
        assert!(root < child);
        assert!(root < grand_child);
    }

    #[test]
    fn rust_keyword_field_uses_raw_ident() {
        let code = json_to_structs(
            r#"{"type":"user","data":{"x":1}}"#,
            "Wrapper",
            TypeLang::Rust,
        )
        .unwrap();
        assert!(code.contains("pub r#type: String,"));
        assert!(code.contains("pub data: Data,"));
    }

    #[test]
    fn rust_integer_and_float_mapping() {
        let code = json_to_structs(
            r#"{"count":1,"ratio":1.5,"big":9223372036854775807}"#,
            "M",
            TypeLang::Rust,
        )
        .unwrap();
        assert!(code.contains("pub count: i64,"));
        assert!(code.contains("pub ratio: f64,"));
    }

    #[test]
    fn mixed_array_merges_to_float() {
        let code = json_to_structs(r#"{"scores":[1,2.5]}"#, "M", TypeLang::Rust).unwrap();
        assert!(code.contains("pub scores: Vec<f64>,"));
    }

    #[test]
    fn unknown_type_fallback_per_language() {
        let json = r#"{"a":null,"b":[null],"c":[1,"x"]}"#;
        let rust = json_to_structs(json, "M", TypeLang::Rust).unwrap();
        assert!(rust.contains("pub a: serde_json::Value,"));
        let java = json_to_structs(json, "M", TypeLang::Java).unwrap();
        assert!(java.contains("private Object a;"));
        let go = json_to_structs(json, "M", TypeLang::Go).unwrap();
        assert!(go.contains("A interface{}"));
    }

    #[test]
    fn name_collision_qualified_by_ancestor() {
        let code = json_to_structs(
            r#"{"data":{"a":1},"info":{"data":{"b":"x"}}}"#,
            "Root",
            TypeLang::Rust,
        )
        .unwrap();
        assert!(code.contains("pub struct Data {"));
        assert!(code.contains("pub struct InfoData {"));
        assert!(code.contains("pub data: InfoData,"));
    }

    #[test]
    fn identical_shape_reused() {
        let code = json_to_structs(
            r#"{"first":{"x":1},"second":{"x":2}}"#,
            "Root",
            TypeLang::Rust,
        )
        .unwrap();
        assert_eq!(code.matches("pub struct First {").count(), 1);
        assert!(code.contains("pub first: First,"));
        assert!(code.contains("pub second: First,"));
    }

    #[test]
    fn java_lombok_and_property_rename() {
        let code = json_to_structs(
            r#"{"user_id":1,"userName":"a"}"#,
            "userInfo",
            TypeLang::Java,
        )
        .unwrap();
        assert!(code.contains("class UserInfo {"));
        assert!(code.contains("@Data"));
        assert!(code.contains("@Builder"));
        assert!(code.contains("private Long userId;"));
        assert!(code.contains("@JsonProperty(\"user_id\")"));
        assert!(code.contains("private String userName;"));
        assert!(!code.contains("@JsonProperty(\"userName\")"));
        assert!(code.contains("import lombok.Data;"));
        assert!(code.contains("import com.fasterxml.jackson.annotation.JsonProperty;"));
    }

    #[test]
    fn java_list_mapping() {
        let code = json_to_structs(r#"{"items":[{"sku":"a"}]}"#, "Root", TypeLang::Java).unwrap();
        assert!(code.contains("private List<Items> items;"));
    }

    #[test]
    fn java_keyword_field_prefixed() {
        let code =
            json_to_structs(r#"{"class":"a","default":"b"}"#, "Root", TypeLang::Java).unwrap();
        assert!(code.contains("private String _class;"));
        assert!(code.contains("@JsonProperty(\"class\")"));
        assert!(code.contains("@JsonProperty(\"default\")"));
    }

    #[test]
    fn go_tags_and_mapping() {
        let code = json_to_structs(
            r#"{"userID":1,"userName":"a","rows":[{"id":2}]}"#,
            "userResponse",
            TypeLang::Go,
        )
        .unwrap();
        assert!(code.contains("type UserResponse struct {"));
        assert!(code.contains("UserID int64 `json:\"userID\"`"));
        assert!(code.contains("UserName string `json:\"userName\"`"));
        assert!(code.contains("Rows []Rows `json:\"rows\"`"));
        assert!(code.contains("type Rows struct {"));
        assert!(code.contains("Id int64 `json:\"id\"`"));
    }

    #[test]
    fn go_digit_leading_key_exported() {
        let code = json_to_structs(r#"{"2fa":"x"}"#, "Root", TypeLang::Go).unwrap();
        assert!(code.contains("N2fa string `json:\"2fa\"`"));
        let rust = json_to_structs(r#"{"2fa":"x"}"#, "Root", TypeLang::Rust).unwrap();
        assert!(rust.contains("#[serde(rename = \"2fa\")]"));
        assert!(rust.contains("pub _2fa: String,"));
    }

    #[test]
    fn root_array_of_objects() {
        let code = json_to_structs(r#"[{"id":1},{"id":2}]"#, "Item", TypeLang::Go).unwrap();
        assert!(code.contains("type Item struct {"));
        assert!(code.contains("Id int64 `json:\"id\"`"));
    }

    #[test]
    fn invalid_json_reports_error() {
        let err = json_to_structs("{bad", "Root", TypeLang::Rust).unwrap_err();
        assert!(matches!(err, CodeGenError::JsonParse(_)));
    }

    #[test]
    fn primitive_root_rejected() {
        let err = json_to_structs("\"just a string\"", "Root", TypeLang::Rust).unwrap_err();
        assert!(matches!(err, CodeGenError::TypeInference(_)));
    }

    #[test]
    fn empty_root_name_rejected() {
        let err = json_to_structs("{}", "  ", TypeLang::Rust).unwrap_err();
        assert!(matches!(err, CodeGenError::TypeInference(_)));
    }

    #[test]
    fn lang_metadata() {
        assert_eq!(TypeLang::from_name("java"), Some(TypeLang::Java));
        assert_eq!(TypeLang::from_name("php"), None);
        assert_eq!(TypeLang::Rust.label(), "Rust (serde)");
        assert_eq!(TypeLang::Go.file_suffix(), "go");
        assert_eq!(TypeLang::all().len(), 3);
    }

    #[test]
    fn empty_object_emits_empty_struct() {
        let rust = json_to_structs("{}", "Empty", TypeLang::Rust).unwrap();
        assert!(rust.contains("pub struct Empty {"));
        assert!(rust.contains("}"));
    }
}
