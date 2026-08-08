//! 断言执行（SPEC §17.4 / §17.5）。

use fox_http::client::HttpResponseData;
use serde_json::Value;

use crate::config::AssertionSpec;

/// 单条断言结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub description: String,
    pub passed: bool,
    /// 失败原因（供失败高亮展示）。
    pub reason: Option<String>,
}

impl Outcome {
    fn pass(description: String) -> Self {
        Outcome {
            description,
            passed: true,
            reason: None,
        }
    }

    fn fail(description: String, reason: String) -> Self {
        Outcome {
            description,
            passed: false,
            reason: Some(reason),
        }
    }
}

/// 断言描述文本。
pub fn describe(a: &AssertionSpec) -> String {
    if let Some(name) = &a.name {
        if !name.is_empty() {
            return name.clone();
        }
    }
    let op = a.op.as_deref().unwrap_or("eq");
    let expected = a
        .expected
        .as_ref()
        .map(v_to_text)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| a.path.clone().unwrap_or_default());
    format!("{} {} {}", a.r#type, op, expected)
}

fn v_to_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// 值是否等于期望文本（数字/布尔/字符串宽松比较）。
fn value_eq(v: &Value, text: &str) -> bool {
    match v {
        Value::String(s) => s == text,
        Value::Number(n) => n.to_string() == text,
        Value::Bool(b) => b.to_string() == text,
        _ => false,
    }
}

/// 取断言的实际值；无匹配返回 None。
fn actual(a: &AssertionSpec, resp: &HttpResponseData, body_value: Option<&Value>) -> Option<Value> {
    match a.r#type.as_str() {
        "status" => Some(Value::Number(resp.status.into())),
        "header" => {
            let name = a.path.as_deref().unwrap_or_default();
            resp.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| Value::String(v.clone()))
        }
        "response_time_ms" => Some(Value::Number(resp.duration_ms.into())),
        "body" => Some(Value::String(resp.body_text())),
        "jsonpath" => {
            let path = a.path.as_deref()?;
            crate::extract::extract_body_json(body_value, path).map(Value::String)
        }
        _ => body_value.cloned(),
    }
}

fn cmp_ops(op: &str, got: f64, want: f64) -> bool {
    match op {
        "gt" => got > want,
        "gte" => got >= want,
        "lt" => got < want,
        _ => got <= want,
    }
}

/// 执行单条断言。
pub fn evaluate(a: &AssertionSpec, resp: &HttpResponseData, body_value: Option<&Value>) -> Outcome {
    let description = describe(a);
    let op = a.op.as_deref().unwrap_or("eq").to_string();
    let expected = a.expected.as_ref().map(v_to_text).unwrap_or_default();

    let Some(actual) = actual(a, resp, body_value) else {
        return match op.as_str() {
            "not_exists" => Outcome::pass(description),
            "exists" => Outcome::fail(description, "值不存在（未获取到实际值）".into()),
            _ => Outcome::fail(
                description,
                "未获取到断言的实际值（JSONPath 无匹配或响应头不存在）".into(),
            ),
        };
    };

    let result = match op.as_str() {
        "eq" => {
            if value_eq(&actual, &expected) {
                Ok(())
            } else {
                Err(format!(
                    "实际值 {} 不等于期望值 {expected}",
                    v_to_text(&actual)
                ))
            }
        }
        "neq" => {
            if value_eq(&actual, &expected) {
                Err(format!("实际值 {} 应不等于 {expected}", v_to_text(&actual)))
            } else {
                Ok(())
            }
        }
        "contains" => {
            let content = v_to_text(&actual);
            if content.contains(&expected) {
                Ok(())
            } else {
                Err(format!("「{content}」不包含「{expected}」"))
            }
        }
        "not_contains" => {
            let content = v_to_text(&actual);
            if content.contains(&expected) {
                Err(format!("「{content}」不应包含「{expected}」"))
            } else {
                Ok(())
            }
        }
        "gt" | "gte" | "lt" | "lte" => {
            let got = to_f64(&actual);
            let want = expected.parse::<f64>();
            match (got, want) {
                (Some(g), Ok(w)) if cmp_ops(&op, g, w) => Ok(()),
                (Some(g), Ok(w)) => Err(format!("实际值 {g} 不满足 {op} {w}")),
                (None, _) => Err(format!("实际值 {} 不是数字", v_to_text(&actual))),
                (_, Err(_)) => Err(format!("期望值「{expected}」不是数字")),
            }
        }
        "exists" => Ok(()),
        "not_exists" => Err(format!("值存在（{}），应不存在", v_to_text(&actual))),
        other => Err(format!("不支持的操作符：{other}")),
    };

    match result {
        Ok(()) => Outcome::pass(description),
        Err(reason) => Outcome::fail(description, reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_resp(
        status: u16,
        headers: Vec<(&str, &str)>,
        body: &str,
        duration_ms: u64,
    ) -> HttpResponseData {
        HttpResponseData {
            status,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Bytes::from(body.to_string()),
            duration_ms,
            size_bytes: body.len(),
            cookies: Vec::new(),
            truncated: false,
        }
    }

    fn spec(t: &str, op: &str, expected: Value, path: Option<&str>) -> AssertionSpec {
        AssertionSpec {
            name: None,
            r#type: t.to_string(),
            path: path.map(|s| s.to_string()),
            op: Some(op.to_string()),
            expected: Some(expected),
        }
    }

    #[test]
    fn status_eq_and_neq() {
        let r = make_resp(200, vec![], "x", 5);
        assert!(evaluate(&spec("status", "eq", json!(200), None), &r, None).passed);
        assert!(!evaluate(&spec("status", "eq", json!(404), None), &r, None).passed);
        assert!(evaluate(&spec("status", "neq", json!(500), None), &r, None).passed);
        // 字符串期望宽松比较
        assert!(evaluate(&spec("status", "eq", json!("200"), None), &r, None).passed);
    }

    #[test]
    fn header_contains_and_not_exists() {
        let r = make_resp(
            201,
            vec![("X-Token", "abc123"), ("Set-Cookie", "sid=1")],
            "x",
            5,
        );
        let a = spec("header", "contains", json!("bc"), Some("x-token")); // 大小写不敏感
        assert!(evaluate(&a, &r, None).passed);
        let missing = AssertionSpec {
            name: None,
            r#type: "header".into(),
            path: Some("X-Nope".into()),
            op: Some("exists".into()),
            expected: None,
        };
        assert!(!evaluate(&missing, &r, None).passed);
        let absent = AssertionSpec {
            name: None,
            r#type: "header".into(),
            path: Some("X-Nope".into()),
            op: Some("not_exists".into()),
            expected: None,
        };
        assert!(evaluate(&absent, &r, None).passed);
    }

    #[test]
    fn jsonpath_eq_and_contains() {
        let body = json!({"data": {"id": 7, "name": "rustfox", "tags": ["a", "b"]}});
        let r = make_resp(200, vec![], &body.to_string(), 3);
        let a = spec("jsonpath", "eq", json!(7), Some("$.data.id"));
        assert!(evaluate(&a, &r, Some(&body)).passed);
        let a2 = spec("jsonpath", "contains", json!("fox"), Some("$.data.name"));
        assert!(evaluate(&a2, &r, Some(&body)).passed);
        // jsonpath 无匹配 → 失败
        let a3 = spec("jsonpath", "eq", json!(7), Some("$.data.nope"));
        let out = evaluate(&a3, &r, None);
        assert!(!out.passed);
        assert!(out.reason.is_some());
    }

    #[test]
    fn numeric_ops() {
        let r = make_resp(200, vec![], "x", 1500);
        let a = spec("response_time_ms", "lt", json!(2000), None);
        assert!(evaluate(&a, &r, None).passed);
        let a2 = spec("response_time_ms", "gt", json!(2000), None);
        assert!(!evaluate(&a2, &r, None).passed);
        let a3 = spec("status", "gte", json!(200), None);
        assert!(evaluate(&a3, &r, None).passed);
    }

    #[test]
    fn body_text_contains() {
        let r = make_resp(200, vec![], "hello world from fox", 1);
        let a = spec("body", "contains", json!("world"), None);
        assert!(evaluate(&a, &r, None).passed);
        let a2 = spec("body", "contains", json!("banana"), None);
        assert!(!evaluate(&a2, &r, None).passed);
    }

    #[test]
    fn describe_name_preferred() {
        let a = AssertionSpec {
            name: Some("接口正常".into()),
            r#type: "status".into(),
            path: None,
            op: Some("eq".into()),
            expected: Some(json!(200)),
        };
        assert_eq!(describe(&a), "接口正常");
    }
}
