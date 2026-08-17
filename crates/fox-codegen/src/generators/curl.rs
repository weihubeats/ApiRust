//! curl 生成器：渲染为可直接执行的 curl 命令。

use crate::engine::CodeGenerator;
use crate::error::CodeGenError;
use crate::model::{ApiBody, ApiDefinition};
use crate::util::{build_url, has_header, merged_headers, pretty_json, sq};

/// curl 生成器。
#[derive(Debug, Clone, Copy, Default)]
pub struct CurlGenerator;

impl CodeGenerator for CurlGenerator {
    fn language_name(&self) -> &'static str {
        "curl"
    }

    fn target_sdk(&self) -> &'static str {
        "curl"
    }

    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let url = build_url(&api.url, &api.query_params);
        let headers = merged_headers(api);

        let mut out = format!("curl -X {} '{}'", api.method, sq(&url));
        for (key, value) in &headers {
            out.push_str(&format!(" \\\n     -H '{}: {}'", sq(key), sq(value)));
        }

        match &api.body {
            ApiBody::None => {}
            ApiBody::Json { raw } => {
                // pretty JSON（shell 单引号内可安全多行）。
                out.push_str(&format!(" \\\n     --data '{}'", sq(&pretty_json(raw))));
            }
            ApiBody::FormData { fields } => {
                for field in fields {
                    out.push_str(&format!(
                        " \\\n     -F '{}={}'",
                        sq(&field.key),
                        sq(&field.value)
                    ));
                }
            }
            ApiBody::Raw { raw } => {
                out.push_str(&format!(" \\\n     --data-raw '{}'", sq(raw)));
            }
        }

        if matches!(api.body, ApiBody::Json { .. }) && !has_header(&headers, "content-type") {
            out.push_str(" \\\n     -H 'Content-Type: application/json'");
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use fox_core::model::HttpMethod;

    use super::*;
    use crate::model::{ApiBody, AuthInfo, KeyValuePair};

    #[test]
    fn json_body_pretty_with_auth_query_and_header() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::POST)
            .query("page", "1")
            .header("X-Custom", "v")
            .body(ApiBody::Json {
                raw: "{\"name\":\"a\"}".into(),
            })
            .auth(AuthInfo::Bearer {
                token: "tok123".into(),
            });

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("curl -X POST 'https://api.example.com/users?page=1'"));
        assert!(code.contains("-H 'Authorization: Bearer tok123'"));
        assert!(code.contains("-H 'X-Custom: v'"));
        assert!(code.contains("--data '{\n  \"name\": \"a\"\n}'"));
        assert!(code.contains("Content-Type: application/json"));
    }

    #[test]
    fn get_without_body_and_query_params() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::GET)
            .query("a", "1")
            .query("b", "a b");

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("curl -X GET 'https://api.example.com/users?a=1&b=a%20b'"));
        assert!(!code.contains("--data"));
    }

    #[test]
    fn query_appended_to_existing_query_string() {
        let api = ApiDefinition::new("https://api.example.com/x?token=1", HttpMethod::GET)
            .query("page", "2");

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("'https://api.example.com/x?token=1&page=2'"));
    }

    #[test]
    fn single_quote_escaped_in_shell_literal() {
        let api =
            ApiDefinition::new("https://api.example.com/s", HttpMethod::GET).header("X-Q", "it's");

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("-H 'X-Q: it'\\''s'"));
    }

    #[test]
    fn basic_auth_encoded() {
        let api = ApiDefinition::new("https://api.example.com/s", HttpMethod::GET).auth(
            AuthInfo::Basic {
                username: "user".into(),
                password: "pass".into(),
            },
        );

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("-H 'Authorization: Basic dXNlcjpwYXNz'"));
    }

    #[test]
    fn apikey_header_auth() {
        let api = ApiDefinition::new("https://api.example.com/m", HttpMethod::GET).auth(
            AuthInfo::ApiKey {
                key: "X-Key".into(),
                value: "v1".into(),
            },
        );

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("-H 'X-Key: v1'"));
    }

    #[test]
    fn explicit_header_overrides_auth() {
        let api = ApiDefinition::new("https://api.example.com/d", HttpMethod::GET)
            .header("authorization", "manual")
            .auth(AuthInfo::Bearer {
                token: "tok".into(),
            });

        let code = CurlGenerator.generate(&api).unwrap();
        assert_eq!(code.matches("Authorization").count(), 1);
        assert!(code.contains("manual"));
    }

    #[test]
    fn formdata_uses_dash_f() {
        let api = ApiDefinition::new("https://api.example.com/upload", HttpMethod::POST).body(
            ApiBody::FormData {
                fields: vec![
                    KeyValuePair::new("name", "张三"),
                    KeyValuePair::new("file", "@/tmp/a.png"),
                ],
            },
        );

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("-F 'name=张三'"));
        assert!(code.contains("-F 'file=@/tmp/a.png'"));
        assert!(!code.contains("--data"));
    }

    #[test]
    fn raw_body_uses_dash_data_raw() {
        let api =
            ApiDefinition::new("https://api.example.com/r", HttpMethod::POST).body(ApiBody::Raw {
                raw: "hello world".into(),
            });

        let code = CurlGenerator.generate(&api).unwrap();
        assert!(code.contains("--data-raw 'hello world'"));
    }

    #[test]
    fn explicit_content_type_not_duplicated() {
        let api = ApiDefinition::new("https://api.example.com/j", HttpMethod::POST)
            .header("Content-Type", "application/vnd.api+json")
            .body(ApiBody::Json { raw: "{}".into() });

        let code = CurlGenerator.generate(&api).unwrap();
        assert_eq!(code.matches("Content-Type").count(), 1);
        assert!(code.contains("application/vnd.api+json"));
    }
}
