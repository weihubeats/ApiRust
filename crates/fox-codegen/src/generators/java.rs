//! Java 生成器：OkHttp 3/4 构建模式，附带单例 `OkHttpClient`。

use crate::engine::CodeGenerator;
use crate::error::CodeGenError;
use crate::model::{ApiBody, ApiDefinition};
use crate::util::{build_url, dq, merged_headers, pretty_json};

/// Java (OkHttp) 生成器。
#[derive(Debug, Clone, Copy, Default)]
pub struct JavaGenerator;

impl CodeGenerator for JavaGenerator {
    fn language_name(&self) -> &'static str {
        "java"
    }

    fn target_sdk(&self) -> &'static str {
        "Java (OkHttp)"
    }

    fn generate(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let url = build_url(&api.url, &api.query_params);
        let headers = merged_headers(api);

        // body 前置语句（已含 8 空格缩进），与 `.method(...)` 中引用的 body 变量配套。
        let mut preamble = String::new();
        let body_ref = match &api.body {
            ApiBody::None => "null".to_string(),
            ApiBody::Json { raw } => {
                let pretty = pretty_json(raw);
                preamble.push_str("        MediaType mediaType = MediaType.parse(\"application/json; charset=utf-8\");\n");
                preamble.push_str(&format!(
                    "        RequestBody body = RequestBody.create(mediaType, \"{}\");\n",
                    dq(&pretty)
                ));
                "body".to_string()
            }
            ApiBody::Raw { raw } => {
                preamble.push_str("        MediaType mediaType = MediaType.parse(\"text/plain; charset=utf-8\");\n");
                preamble.push_str(&format!(
                    "        RequestBody body = RequestBody.create(mediaType, \"{}\");\n",
                    dq(raw)
                ));
                "body".to_string()
            }
            ApiBody::FormData { fields } => {
                let mut chain = vec![".setType(MultipartBody.FORM)".to_string()];
                for field in fields {
                    chain.push(format!(
                        ".addFormDataPart(\"{}\", \"{}\")",
                        dq(&field.key),
                        dq(&field.value)
                    ));
                }
                chain.push(".build();".to_string());
                preamble.push_str(
                    "        MultipartBody.Builder formBuilder = new MultipartBody.Builder()\n",
                );
                for step in chain {
                    preamble.push_str("                ");
                    preamble.push_str(&step);
                    preamble.push('\n');
                }
                preamble.push_str("        RequestBody body = formBuilder;\n");
                "body".to_string()
            }
        };

        let mut out = String::from(
            "import okhttp3.*;\nimport java.io.IOException;\nimport java.util.concurrent.TimeUnit;\n\n",
        );
        out.push_str("public class Main {\n\n");
        // 抽象的 HttpClient 单例：连接/读超时 30s，复用连接池。
        out.push_str(
            "    private static final OkHttpClient HTTP_CLIENT = new OkHttpClient.Builder()\n",
        );
        out.push_str("            .connectTimeout(30, TimeUnit.SECONDS)\n");
        out.push_str("            .readTimeout(30, TimeUnit.SECONDS)\n");
        out.push_str("            .build();\n\n");

        out.push_str("    public static void main(String[] args) throws IOException {\n");
        out.push_str(&format!("        String url = \"{}\";\n", dq(&url)));
        if !preamble.is_empty() {
            out.push('\n');
            out.push_str(&preamble);
        }
        out.push('\n');
        out.push_str("        Request request = new Request.Builder()\n");
        out.push_str("                .url(url)\n");
        out.push_str(&format!(
            "                .method(\"{}\", {body_ref})\n",
            api.method
        ));
        for (key, value) in &headers {
            out.push_str(&format!(
                "                .addHeader(\"{}\", \"{}\")\n",
                dq(key),
                dq(value)
            ));
        }
        out.push_str("                .build();\n\n");

        out.push_str(
            "        try (Response response = HTTP_CLIENT.newCall(request).execute()) {\n",
        );
        out.push_str("            if (!response.isSuccessful()) {\n");
        out.push_str("                throw new IOException(\"Unexpected code \" + response);\n");
        out.push_str("            }\n");
        out.push_str("            String responseBody = response.body() != null ? response.body().string() : \"\";\n");
        out.push_str("            System.out.println(response.code() + \" \" + responseBody);\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
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
    fn json_body_pretty_and_singleton_client() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::POST)
            .body(ApiBody::Json {
                raw: "{\"name\":\"a\"}".into(),
            })
            .auth(AuthInfo::Bearer {
                token: "tok".into(),
            });

        let code = JavaGenerator.generate(&api).unwrap();
        assert!(code.contains(
            "private static final OkHttpClient HTTP_CLIENT = new OkHttpClient.Builder()"
        ));
        assert!(code.contains(".connectTimeout(30, TimeUnit.SECONDS)"));
        assert!(code.contains("MediaType.parse(\"application/json; charset=utf-8\")"));
        assert!(code.contains("RequestBody body = RequestBody.create(mediaType, \"{\\n  \\\"name\\\": \\\"a\\\"\\n}\");"));
        assert!(code.contains(".method(\"POST\", body)"));
        assert!(code.contains(".addHeader(\"Authorization\", \"Bearer tok\")"));
        assert!(code.contains("HTTP_CLIENT.newCall(request).execute()"));
        assert!(code.contains("throw new IOException"));
    }

    #[test]
    fn get_without_body_uses_null() {
        let api = ApiDefinition::new("https://api.example.com/g", HttpMethod::GET).query("a", "1");

        let code = JavaGenerator.generate(&api).unwrap();
        assert!(code.contains(".method(\"GET\", null)"));
        assert!(!code.contains("RequestBody body"));
    }

    #[test]
    fn formdata_uses_multipart_builder() {
        let api = ApiDefinition::new("https://api.example.com/upload", HttpMethod::POST).body(
            ApiBody::FormData {
                fields: vec![
                    KeyValuePair::new("name", "张三"),
                    KeyValuePair::new("tag", "a\"b"),
                ],
            },
        );

        let code = JavaGenerator.generate(&api).unwrap();
        assert!(code.contains("MultipartBody.Builder formBuilder = new MultipartBody.Builder()"));
        assert!(code.contains(".setType(MultipartBody.FORM)"));
        assert!(code.contains(".addFormDataPart(\"name\", \"张三\")"));
        assert!(code.contains(".addFormDataPart(\"tag\", \"a\\\"b\")"));
        assert!(code.contains("RequestBody body = formBuilder;"));
    }

    #[test]
    fn raw_body_with_newline_escaped() {
        let api =
            ApiDefinition::new("https://api.example.com/r", HttpMethod::POST).body(ApiBody::Raw {
                raw: "l1\nl2 \"q\"".into(),
            });

        let code = JavaGenerator.generate(&api).unwrap();
        assert!(code.contains("RequestBody.create(mediaType, \"l1\\nl2 \\\"q\\\"\")"));
        assert!(!code.contains("l1\nl2"));
    }

    #[test]
    fn json_quotes_escaped() {
        let api =
            ApiDefinition::new("https://api.example.com/q", HttpMethod::POST).body(ApiBody::Json {
                raw: "{\"msg\":\"a\\\"b\\\\c\"}".into(),
            });

        let code = JavaGenerator.generate(&api).unwrap();
        // 原 JSON 中的 `\"` 与 `\\` 在 Java 字符串里各再加一层转义。
        assert!(code.contains("a\\\\\\\"b\\\\\\\\c"));
    }
}
