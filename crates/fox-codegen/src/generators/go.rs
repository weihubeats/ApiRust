//! Go 生成器：标准库 `net/http`，包含 Context 超时与错误处理。
//!
//! 重点展示「按需 import」：只有实际使用的包才会出现在 `import` 块中
//! （Go 编译器对未使用的 import 是硬错误，这是生成代码合法性的关键点）。

use crate::engine::CodeGenerator;
use crate::error::CodeGenError;
use crate::model::{ApiBody, ApiDefinition};
use crate::util::{build_url, dq, has_header, merged_headers, pretty_json};

/// Go (net/http) 生成器。
#[derive(Debug, Clone, Copy, Default)]
pub struct GoGenerator;

impl CodeGenerator for GoGenerator {
    fn language_name(&self) -> &'static str {
        "go"
    }

    fn target_sdk(&self) -> &'static str {
        "Go (net/http)"
    }

    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let url = build_url(&api.url, &api.query_params);
        let headers = merged_headers(api);

        let mut imports = vec!["context", "fmt", "io", "net/http", "time"];
        let mut body_lines: Vec<String> = Vec::new();
        // 请求体表达式：拼进 `http.NewRequest(method, url, <expr>)`。
        // 显式打分以保证 match 各分支都有赋值（穷尽性保证安全）。
        let body_arg = match &api.body {
            ApiBody::None => "nil".to_string(),
            ApiBody::Json { raw } => {
                imports.push("bytes");
                let pretty = pretty_json(raw);
                if pretty.contains('`') {
                    // 反引号原始字符串无法容纳反引号，退化为双引号字符串并转义。
                    format!("bytes.NewBufferString(\"{}\")", dq(&pretty))
                } else {
                    // Go raw string literal（反引号）：body 可原样多行嵌入。
                    format!("bytes.NewBufferString(`\n{}\n`)", pretty)
                }
            }
            ApiBody::Raw { raw } => {
                imports.push("bytes");
                format!("bytes.NewBufferString(\"{}\")", dq(raw))
            }
            ApiBody::FormData { fields } => {
                imports.push("bytes");
                imports.push("mime/multipart");
                body_lines.push("var bodyBuf bytes.Buffer".into());
                body_lines.push("writer := multipart.NewWriter(&bodyBuf)".into());
                for field in fields {
                    body_lines.push(format!(
                        "_ = writer.WriteField(\"{}\", \"{}\")",
                        dq(&field.key),
                        dq(&field.value)
                    ));
                }
                body_lines.push("_ = writer.Close()".into());
                "&bodyBuf".to_string()
            }
        };
        // Content-Type：JSON 固定；FormData 由 multipart 写入器给出（含 boundary）。
        let content_type = match &api.body {
            ApiBody::Json { .. } => Some("\"application/json\"".to_string()),
            ApiBody::FormData { .. } => Some("writer.FormDataContentType()".to_string()),
            _ => None,
        };

        imports.sort_unstable();
        let mut out = String::from("package main\n\nimport (\n");
        for import in imports {
            out.push_str(&format!("    \"{import}\"\n"));
        }
        out.push_str(")\n\n");
        out.push_str("func main() {\n");

        out.push_str(&format!("    url := \"{}\"\n", dq(&url)));
        for line in &body_lines {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
        if !body_lines.is_empty() {
            out.push('\n');
        }

        out.push_str(&format!(
            "    req, err := http.NewRequest(\"{}\", url, {})\n",
            api.method, body_arg
        ));
        out.push_str("    if err != nil {\n        fmt.Println(\"构建请求失败:\", err)\n        return\n    }\n");
        for (key, value) in &headers {
            out.push_str(&format!(
                "    req.Header.Set(\"{}\", \"{}\")\n",
                dq(key),
                dq(value)
            ));
        }
        if let Some(ct) = &content_type {
            if !has_header(&headers, "content-type") {
                out.push_str(&format!("    req.Header.Set(\"Content-Type\", {ct})\n"));
            }
        }

        out.push_str(
            "\n    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)\n",
        );
        out.push_str("    defer cancel()\n");
        out.push_str("    req = req.WithContext(ctx)\n\n");

        out.push_str("    resp, err := http.DefaultClient.Do(req)\n");
        out.push_str(
            "    if err != nil {\n        fmt.Println(\"请求失败:\", err)\n        return\n    }\n",
        );
        out.push_str("    defer resp.Body.Close()\n\n");

        out.push_str("    respBody, err := io.ReadAll(resp.Body)\n");
        out.push_str("    if err != nil {\n        fmt.Println(\"读取响应失败:\", err)\n        return\n    }\n");
        out.push_str("    fmt.Println(resp.Status, string(respBody))\n");
        out.push_str("}\n");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use fox_core::model::HttpMethod;

    use super::*;
    use crate::model::{ApiBody, AuthInfo, KeyValuePair};

    #[test]
    fn json_body_uses_raw_string_literal() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::POST)
            .body(ApiBody::Json {
                raw: "{\"name\":\"a\"}".into(),
            })
            .auth(AuthInfo::Bearer {
                token: "tok".into(),
            });

        let code = GoGenerator.generate(&api).unwrap();
        assert!(code.contains("import"));
        assert!(code.contains("\"bytes\""));
        assert!(code.contains("bytes.NewBufferString(`"));
        assert!(code.contains("  \"name\": \"a\""));
        assert!(code.contains("http.NewRequest(\"POST\", url, bytes.NewBufferString("));
        assert!(code.contains("req.Header.Set(\"Authorization\", \"Bearer tok\")"));
        assert!(code.contains("req.Header.Set(\"Content-Type\", \"application/json\")"));
        assert!(code.contains("context.WithTimeout(context.Background(), 30*time.Second)"));
        assert!(code.contains("req = req.WithContext(ctx)"));
        assert!(code.contains("resp, err := http.DefaultClient.Do(req)"));
    }

    #[test]
    fn get_without_body_uses_nil() {
        let api =
            ApiDefinition::new("https://api.example.com/g?x=1", HttpMethod::GET).query("page", "2");

        let code = GoGenerator.generate(&api).unwrap();
        assert!(code.contains("http.NewRequest(\"GET\", url, nil)"));
        assert!(code.contains("https://api.example.com/g?x=1&page=2"));
        assert!(!code.contains("\"bytes\""));
        assert!(!code.contains("\"mime/multipart\""));
    }

    #[test]
    fn body_with_backtick_falls_back_to_escaped_string() {
        let api =
            ApiDefinition::new("https://api.example.com/s", HttpMethod::POST).body(ApiBody::Json {
                raw: "{\"q\":\"a`b\"}".into(),
            });

        let code = GoGenerator.generate(&api).unwrap();
        assert!(code.contains("bytes.NewBufferString(\""));
        assert!(!code.contains("bytes.NewBufferString(`"));
    }

    #[test]
    fn formdata_uses_multipart_writer() {
        let api = ApiDefinition::new("https://api.example.com/upload", HttpMethod::POST).body(
            ApiBody::FormData {
                fields: vec![
                    KeyValuePair::new("name", "张三"),
                    KeyValuePair::new("tag", "a\"b"),
                ],
            },
        );

        let code = GoGenerator.generate(&api).unwrap();
        assert!(code.contains("\"mime/multipart\""));
        assert!(code.contains("writer := multipart.NewWriter(&bodyBuf)"));
        assert!(code.contains("writer.WriteField(\"name\", \"张三\")"));
        assert!(code.contains("writer.WriteField(\"tag\", \"a\\\"b\")"));
        assert!(code.contains("req.Header.Set(\"Content-Type\", writer.FormDataContentType())"));
        assert!(code.contains("http.NewRequest(\"POST\", url, &bodyBuf)"));
    }

    #[test]
    fn raw_body_escaped() {
        let api =
            ApiDefinition::new("https://api.example.com/r", HttpMethod::POST).body(ApiBody::Raw {
                raw: "line1\nline2 \"quoted\"".into(),
            });

        let code = GoGenerator.generate(&api).unwrap();
        assert!(code.contains("line1\\nline2 \\\"quoted\\\""));
        assert!(!code.contains("line1\nline2"));
    }

    #[test]
    fn explicit_content_type_not_overridden() {
        let api = ApiDefinition::new("https://api.example.com/j", HttpMethod::POST)
            .header("Content-Type", "application/vnd.api+json")
            .body(ApiBody::Json { raw: "{}".into() });

        let code = GoGenerator.generate(&api).unwrap();
        assert_eq!(code.matches("Content-Type").count(), 1);
    }
}
