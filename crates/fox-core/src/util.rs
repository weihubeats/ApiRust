//! 通用工具：时间、UUID、URL 构建、JSON 格式化。

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppError;

/// 当前 UTC 时间。
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// 新的 UUID v4。
pub fn new_uuid() -> Uuid {
    Uuid::new_v4()
}

/// 判断字符串是否以 http:// 或 https:// 开头。
pub fn is_absolute_url(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("http://") || t.starts_with("https://")
}

/// 路径变量替换：支持 `{id}` 与 `{{id}}` 两种写法，按 key 长度降序替换避免前缀冲突。
pub fn replace_path_variables(path: &str, path_vars: &HashMap<String, String>) -> String {
    let mut result = path.to_string();
    let mut keys: Vec<&String> = path_vars.keys().collect();
    keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for key in keys {
        let value = &path_vars[key];
        result = result.replace(&format!("{{{{{key}}}}}"), value);
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

/// URL 拼接规则（SPEC 第 13 节）：
/// 1. path 是完整 URL 时直接使用；
/// 2. 否则与 base_url 拼接；
/// 3. path 变量必须先替换。
pub fn build_url(
    base_url: Option<&str>,
    path: &str,
    path_vars: &HashMap<String, String>,
) -> String {
    let rendered_path = replace_path_variables(path, path_vars);
    if is_absolute_url(&rendered_path) {
        return rendered_path;
    }
    let p = rendered_path.trim().trim_start_matches('/');
    match base_url {
        Some(base) => {
            let base = base.trim().trim_end_matches('/');
            if base.is_empty() {
                format!("/{p}")
            } else {
                format!("{base}/{p}")
            }
        }
        None => format!("/{p}"),
    }
}

/// 格式化 JSON 文本；非法时返回错误。
pub fn format_json(raw: &str) -> Result<String, AppError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(AppError::Json)?;
    serde_json::to_string_pretty(&value).map_err(AppError::Json)
}

/// 判断 Content-Type 是否为 JSON。
pub fn is_json_content_type(content_type: &str) -> bool {
    let ct = content_type.split(';').next().unwrap_or("").trim();
    ct == "application/json" || ct == "text/json" || ct.ends_with("+json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn absolute_url_used_directly() {
        let v = vars(&[]);
        let out = build_url(
            Some("https://base.com"),
            "https://api.example.com/users",
            &v,
        );
        assert_eq!(out, "https://api.example.com/users");
    }

    #[test]
    fn relative_path_joined_with_base() {
        let v = vars(&[]);
        assert_eq!(
            build_url(Some("https://api.example.com"), "/users", &v),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn trailing_slash_normalized() {
        let v = vars(&[]);
        assert_eq!(
            build_url(Some("https://api.example.com/"), "users", &v),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn path_variables_replaced() {
        let v = vars(&[("id", "10")]);
        assert_eq!(
            build_url(Some("https://api.example.com"), "/users/{id}", &v),
            "https://api.example.com/users/10"
        );
    }

    #[test]
    fn path_variables_double_braces_replaced() {
        let v = vars(&[("id", "10")]);
        assert_eq!(
            build_url(Some("https://api.example.com"), "/users/{{id}}", &v),
            "https://api.example.com/users/10"
        );
    }

    #[test]
    fn no_base_url_keeps_path() {
        let v = vars(&[]);
        assert_eq!(build_url(None, "/users", &v), "/users");
    }

    #[test]
    fn format_json_pretty() {
        let out = format_json("{\"a\":1}").unwrap();
        assert!(out.contains("\n  \"a\": 1"));
    }

    #[test]
    fn format_json_invalid() {
        assert!(format_json("{not json}").is_err());
    }

    #[test]
    fn json_content_type_detection() {
        assert!(is_json_content_type("application/json"));
        assert!(is_json_content_type("application/json; charset=utf-8"));
        assert!(is_json_content_type("application/problem+json"));
        assert!(!is_json_content_type("text/html"));
    }
}
