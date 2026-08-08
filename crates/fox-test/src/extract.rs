//! 变量提取（SPEC §17.3 extract）：body JSONPath 或 header。

use std::collections::HashMap;

use fox_http::client::HttpResponseData;
use jsonpath_rust::{JsonPathFinder, JsonPathInst};
use serde_json::Value;

use crate::config::ExtractSpec;

/// JSONPath 提取：取第一个匹配项，转字符串；无匹配返回 None。
pub fn extract_body_json(body_value: Option<&Value>, path: &str) -> Option<String> {
    let json = body_value?;
    let inst: JsonPathInst = path.parse().ok()?;
    let finder = JsonPathFinder::new(Box::new(json.clone()), Box::new(inst));
    let matched = finder.find_slice().into_iter().next()?;
    let value = matched.to_data();
    match value {
        Value::String(s) => Some(s),
        // Null 视为未提取到（缺字段）。
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// 按 extract 配置提取变量。
pub fn extract_variables(
    specs: &[ExtractSpec],
    resp: &HttpResponseData,
    body_value: Option<&Value>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for spec in specs {
        let value = match spec.from.as_str() {
            "header" => resp
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&spec.path))
                .map(|(_, v)| v.clone()),
            _ => extract_body_json(body_value, &spec.path),
        };
        if let Some(value) = value {
            out.insert(spec.name.clone(), value);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn resp_with(body: &str, headers: Vec<(&str, &str)>) -> HttpResponseData {
        HttpResponseData {
            status: 200,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Bytes::from(body.to_string()),
            duration_ms: 1,
            size_bytes: body.len(),
            cookies: Vec::new(),
            truncated: false,
        }
    }

    #[test]
    fn jsonpath_scalar_and_nested() {
        let v = json!({"id": "u1", "nested": {"x": 42}, "list": [{"k": 1}]});
        assert_eq!(extract_body_json(Some(&v), "$.id"), Some("u1".into()));
        assert_eq!(extract_body_json(Some(&v), "$.nested.x"), Some("42".into()));
        assert_eq!(extract_body_json(Some(&v), "$.list[0].k"), Some("1".into()));
        assert_eq!(extract_body_json(Some(&v), "$.missing"), None);
    }

    #[test]
    fn numbers_become_strings() {
        let v = json!({"n": 7.5, "flag": true});
        assert_eq!(extract_body_json(Some(&v), "$.n"), Some("7.5".into()));
        assert_eq!(extract_body_json(Some(&v), "$.flag"), Some("true".into()));
    }

    #[test]
    fn invalid_path_returns_none() {
        let v = json!({"a": 1});
        assert_eq!(extract_body_json(Some(&v), "$$bad("), None);
        assert_eq!(extract_body_json(None, "$.a"), None);
    }

    #[test]
    fn header_and_body_extraction() {
        let r = resp_with(r#"{"token":"t-9"}"#, vec![("X-Request-Id", "req-1")]);
        let specs = vec![
            ExtractSpec {
                name: "tid".into(),
                from: "body".into(),
                path: "$.token".into(),
            },
            ExtractSpec {
                name: "rid".into(),
                from: "header".into(),
                path: "x-request-id".into(),
            },
            ExtractSpec {
                name: "none".into(),
                from: "body".into(),
                path: "$.missing".into(),
            },
        ];
        let body_value = serde_json::from_str::<Value>(r#"{"token":"t-9"}"#).ok();
        let vars = extract_variables(&specs, &r, body_value.as_ref());
        assert_eq!(vars.get("tid"), Some(&"t-9".to_string()));
        assert_eq!(vars.get("rid"), Some(&"req-1".to_string()));
        assert!(!vars.contains_key("none"));
    }
}
