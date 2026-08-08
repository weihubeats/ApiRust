//! 测试配置解析（SPEC §17.3）。

use serde::{Deserialize, Serialize};

/// 变量设置：pre_request。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetVariable {
    pub name: String,
    pub value: String,
}

/// 变量提取：extract。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractSpec {
    pub name: String,
    /// "body" 或 "header"。
    #[serde(default = "default_extract_from")]
    pub from: String,
    /// body 时是 JSONPath（如 $.id），header 时是头名。
    pub path: String,
}

fn default_extract_from() -> String {
    "body".to_string()
}

/// 断言项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionSpec {
    #[serde(default)]
    pub name: Option<String>,
    /// status | header | body | jsonpath | response_time_ms
    pub r#type: String,
    /// header 时是头名；jsonpath 时是路径；body 时忽略。
    #[serde(default)]
    pub path: Option<String>,
    /// eq | neq | contains | not_contains | gt | gte | lt | lte | exists | not_exists
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub expected: Option<serde_json::Value>,
}

/// 完整测试规格（对应 request_json 里的 "tests"）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TestSpec {
    #[serde(default)]
    pub pre_request: Vec<SetVariable>,
    #[serde(default)]
    pub extract: Vec<ExtractSpec>,
    #[serde(default)]
    pub assertions: Vec<AssertionSpec>,
}

impl TestSpec {
    /// 从 request_json.tests 解析；缺失或非法返回 None（非法时收集原因）。
    pub fn from_request_value(value: Option<&serde_json::Value>) -> Result<TestSpec, String> {
        let Some(value) = value else {
            return Ok(TestSpec::default());
        };
        if value.is_null() {
            return Ok(TestSpec::default());
        }
        serde_json::from_value(value.clone()).map_err(|e| format!("测试配置解析失败：{e}"))
    }

    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    pub fn is_empty(&self) -> bool {
        self.pre_request.is_empty() && self.extract.is_empty() && self.assertions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_absent_and_null() {
        assert!(TestSpec::from_request_value(None).unwrap().is_empty());
        assert!(TestSpec::from_request_value(Some(&serde_json::Value::Null))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parse_spec_example() {
        let v = json!({
            "pre_request": [
                {"type": "set_variable", "name": "timestamp", "value": "{{$timestamp}}"}
            ],
            "extract": [
                {"name": "userId", "from": "body", "path": "$.id"}
            ],
            "assertions": [
                {"type": "status", "op": "eq", "expected": 200},
                {"type": "jsonpath", "path": "$.name", "op": "contains", "expected": "test"},
                {"type": "response_time_ms", "op": "lt", "expected": 2000}
            ]
        });
        let spec = TestSpec::from_request_value(Some(&v)).unwrap();
        assert_eq!(spec.pre_request.len(), 1);
        assert_eq!(spec.pre_request[0].name, "timestamp");
        assert_eq!(spec.extract.len(), 1);
        assert_eq!(spec.extract[0].path, "$.id");
        assert_eq!(spec.assertions.len(), 3);
        assert_eq!(spec.assertions[2].r#type, "response_time_ms");
        assert_eq!(spec.assertions[2].op.as_deref(), Some("lt"));
        // roundtrip
        let back = TestSpec::from_request_value(Some(&spec.to_value())).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn invalid_json_fails() {
        assert!(TestSpec::from_request_value(Some(&json!({"assertions": [{"type": 1}]}))).is_err());
    }
}
