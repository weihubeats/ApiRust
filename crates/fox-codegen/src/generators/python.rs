//! Python 生成器：干净、Pythonic 的 `requests` 调用。
//!
//! JSON 请求体被递归渲染为 Python 字典/列表字面量（`true` → `True`、
//! `null` → `None`），字符串值统一单引号转义——从根上规避「JSON 字符串
//! 嵌入 Python 字符串」的双重转义问题。

use crate::engine::CodeGenerator;
use crate::error::CodeGenError;
use crate::model::{ApiBody, ApiDefinition};
use crate::util::{build_url, dq, merged_headers};

/// Python (requests) 生成器。
#[derive(Debug, Clone, Copy, Default)]
pub struct PythonGenerator;

impl CodeGenerator for PythonGenerator {
    fn language_name(&self) -> &'static str {
        "python"
    }

    fn target_sdk(&self) -> &'static str {
        "Python (requests)"
    }

    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let url = build_url(&api.url, &api.query_params);
        let headers = merged_headers(api);

        let mut out = String::from("import requests\n\n");
        out.push_str(&format!("url = \"{}\"\n", dq(&url)));

        if !headers.is_empty() {
            out.push('\n');
            out.push_str("headers = {\n");
            for (key, value) in &headers {
                out.push_str(&format!("    \"{}\": \"{}\",\n", dq(key), dq(value)));
            }
            out.push_str("}\n");
        }

        let mut call_args = vec!["url".to_string()];
        if !headers.is_empty() {
            call_args.push("headers=headers".to_string());
        }

        match &api.body {
            ApiBody::None => {}
            ApiBody::Json { raw } => {
                let literal = match serde_json::from_str::<serde_json::Value>(raw) {
                    Ok(value) => json_to_python(&value, 0),
                    // 非法 JSON：退化为双引号字符串，`json=` 依然可序列化。
                    Err(_) => format!("\"{}\"", dq(raw)),
                };
                out.push('\n');
                out.push_str("payload = ");
                out.push_str(&literal);
                out.push('\n');
                call_args.push("json=payload".to_string());
            }
            ApiBody::FormData { fields } => {
                out.push_str("\nfiles = {\n");
                for field in fields {
                    out.push_str(&format!(
                        "    '{}': '{}',\n",
                        py_sq(&field.key),
                        py_sq(&field.value)
                    ));
                }
                out.push_str("}\n");
                call_args.push("files=files".to_string());
            }
            ApiBody::Raw { raw } => {
                out.push_str(&format!("\npayload = \"{}\"\n", dq(raw)));
                call_args.push("data=payload".to_string());
            }
        }

        out.push_str(&format!(
            "\nresp = requests.{}({})\n",
            api.method.as_str().to_lowercase(),
            call_args.join(", ")
        ));
        out.push_str("resp.raise_for_status()\n");
        out.push_str("print(resp.status_code, resp.text)\n");
        Ok(out)
    }
}

/// 转义 Python 单引号字符串常量（`files` 字典的键值用）。
fn py_sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// JSON 值 → Python 字面量（递归，多行缩进 4 空格）。
fn json_to_python(value: &serde_json::Value, depth: usize) -> String {
    let indent = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", py_sq(s)),
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return "[]".to_string();
            }
            let mut out = String::from("[\n");
            for item in items {
                out.push_str(&inner);
                out.push_str(&json_to_python(item, depth + 1));
                out.push_str(",\n");
            }
            out.push_str(&indent);
            out.push(']');
            out
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut out = String::from("{\n");
            for (key, item) in map {
                out.push_str(&inner);
                out.push_str(&format!("'{}': ", py_sq(key)));
                out.push_str(&json_to_python(item, depth + 1));
                out.push_str(",\n");
            }
            out.push_str(&indent);
            out.push('}');
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use fox_core::model::HttpMethod;

    use super::*;
    use crate::model::{ApiBody, AuthInfo, KeyValuePair};

    #[test]
    fn json_body_rendered_as_python_literal() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::POST)
            .body(ApiBody::Json {
                raw: "{\"name\":\"fox\",\"active\":true,\"count\":2,\"ratio\":1.5,\"tags\":[\"a\",\"b\"],\"extra\":null}"
                    .into(),
            })
            .auth(AuthInfo::Bearer {
                token: "tok".into(),
            });

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("'name': 'fox'"));
        assert!(code.contains("'active': True"));
        assert!(code.contains("'count': 2"));
        assert!(code.contains("'ratio': 1.5"));
        assert!(code.contains("'tags': ["));
        assert!(code.contains("'extra': None"));
        assert!(code.contains("resp = requests.post(url, headers=headers, json=payload)"));
        assert!(code.contains("resp.raise_for_status()"));
        // 不出现 JSON 原始语法残留。
        assert!(!code.contains("\"name\":"));
    }

    #[test]
    fn single_quote_escaped_in_python_literal() {
        let api =
            ApiDefinition::new("https://api.example.com/q", HttpMethod::POST).body(ApiBody::Json {
                raw: "{\"msg\":\"it's ok \\\\\\\\ back\"}".into(),
            });

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("'msg': 'it\\'s ok \\\\\\\\ back'"));
    }

    #[test]
    fn invalid_json_falls_back_to_raw_string() {
        let api =
            ApiDefinition::new("https://api.example.com/q", HttpMethod::POST).body(ApiBody::Json {
                raw: "not-json\"content".into(),
            });

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("payload = \"not-json\\\"content\""));
        assert!(code.contains("json=payload"));
    }

    #[test]
    fn formdata_uses_files_dict_with_single_quotes() {
        let api = ApiDefinition::new("https://api.example.com/upload", HttpMethod::POST).body(
            ApiBody::FormData {
                fields: vec![
                    KeyValuePair::new("name", "张三"),
                    KeyValuePair::new("quote", "it's"),
                ],
            },
        );

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("files = {"));
        assert!(code.contains("'name': '张三'"));
        assert!(code.contains("'quote': 'it\\'s'"));
        assert!(code.contains("resp = requests.post(url, files=files)"));
    }

    #[test]
    fn raw_body_with_newline_escaped() {
        let api =
            ApiDefinition::new("https://api.example.com/r", HttpMethod::POST).body(ApiBody::Raw {
                raw: "l1\nl2".into(),
            });

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("payload = \"l1\\nl2\""));
        assert!(code.contains("data=payload"));
    }

    #[test]
    fn get_without_body_and_no_headers() {
        let api = ApiDefinition::new("https://api.example.com/g", HttpMethod::GET).query("a", "1");

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(!code.contains("headers"));
        assert!(code.contains("resp = requests.get(url)"));
    }

    #[test]
    fn header_values_escaped_in_double_quotes() {
        let api = ApiDefinition::new("https://api.example.com/h", HttpMethod::GET)
            .header("X-Com", "say \"hi\"\nnext");

        let code = PythonGenerator.generate(&api).unwrap();
        assert!(code.contains("\"X-Com\": \"say \\\"hi\\\"\\nnext\""));
    }
}
