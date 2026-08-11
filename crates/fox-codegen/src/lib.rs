//! 客户端代码生成（M13）。
//!
//! 支持 curl / Python (requests) / JavaScript (fetch) / Go (net/http) /
//! Java (OkHttp) / PHP (cURL)。
//! URL 传入时即为渲染后的完整地址（含变量与环境替换）。

use base64::Engine;
use fox_core::model::{
    ApiKeyLocation, AuthSpec, BodySpec, HttpMethod, KeyValue, MultipartField, MultipartValueType,
};

/// 目标语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Curl,
    Python,
    JavaScript,
    Go,
    Java,
    Php,
}

impl Lang {
    pub fn label(&self) -> &'static str {
        match self {
            Lang::Curl => "curl",
            Lang::Python => "Python (requests)",
            Lang::JavaScript => "JavaScript (fetch)",
            Lang::Go => "Go (net/http)",
            Lang::Java => "Java (OkHttp)",
            Lang::Php => "PHP (cURL)",
        }
    }

    pub fn from_str_cn(s: &str) -> Option<Self> {
        match s {
            "curl" => Some(Lang::Curl),
            "python" => Some(Lang::Python),
            "js" => Some(Lang::JavaScript),
            "go" => Some(Lang::Go),
            "java" => Some(Lang::Java),
            "php" => Some(Lang::Php),
            _ => None,
        }
    }
}

/// 生成入参。
pub struct GenRequest<'a> {
    pub method: &'a HttpMethod,
    pub url: &'a str,
    /// 请求头（已启用的）。
    pub headers: &'a [KeyValue],
    pub body: &'a BodySpec,
    pub auth: &'a AuthSpec,
}

/// 认证 → 附加请求头。
fn auth_headers(auth: &AuthSpec) -> Vec<(String, String)> {
    match auth {
        AuthSpec::None => Vec::new(),
        AuthSpec::Bearer { token } if !token.is_empty() => {
            vec![("Authorization".into(), format!("Bearer {token}"))]
        }
        AuthSpec::Basic { username, password } => {
            let raw = format!("{username}:{password}");
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
            vec![("Authorization".into(), format!("Basic {encoded}"))]
        }
        AuthSpec::ApiKey {
            key,
            value,
            location: ApiKeyLocation::Header,
        } if !key.trim().is_empty() && !value.is_empty() => vec![(key.clone(), value.clone())],
        _ => Vec::new(),
    }
}

/// 生成代码。
pub fn render<'a>(lang: Lang, req: &GenRequest<'a>) -> String {
    let mut headers: Vec<(String, String)> = auth_headers(req.auth);
    headers.extend(
        req.headers
            .iter()
            .filter(|kv| kv.enabled && !kv.key.trim().is_empty())
            .map(|kv| (kv.key.trim().to_string(), kv.value.clone())),
    );
    let mut merged: Vec<(String, String)> = Vec::new();
    for (k, v) in headers {
        if let Some(existing) = merged
            .iter_mut()
            .find(|(ek, _)| ek.eq_ignore_ascii_case(&k))
        {
            existing.1 = v;
        } else {
            merged.push((k, v));
        }
    }
    let m = req.method;
    let u = req.url;
    match lang {
        Lang::Curl => render_curl(m, u, &merged, req.body),
        Lang::Python => render_python(m, u, &merged, req.body),
        Lang::JavaScript => render_js(m, u, &merged, req.body),
        Lang::Go => render_go(m, u, &merged, req.body),
        Lang::Java => render_java(m, u, &merged, req.body),
        Lang::Php => render_php(m, u, &merged, req.body),
    }
}

/// (body 文本, 内容类型, multipart 字段)
fn body_parts(body: &BodySpec) -> (String, Option<&'static str>, Option<&Vec<MultipartField>>) {
    match body {
        BodySpec::None => (String::new(), None, None),
        BodySpec::Json { raw } => (raw.clone(), Some("application/json"), None),
        BodySpec::Text { raw } => (raw.clone(), None, None),
        BodySpec::UrlEncoded { fields } => {
            let parts: Vec<String> = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| {
                    format!(
                        "{}={}",
                        encode_component(&f.key),
                        encode_component(&f.value)
                    )
                })
                .collect();
            (
                parts.join("&"),
                Some("application/x-www-form-urlencoded"),
                None,
            )
        }
        BodySpec::Multipart { fields } => (String::new(), None, Some(fields)),
    }
}

/// RFC 3986 表单编码（Component 规则）。
fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    const HEX: &[u8] = b"0123456789ABCDEF";
    for b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_' || *b == b'.' || *b == b'~' {
            out.push(*b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xF) as usize] as char);
        }
    }
    out
}

/// 转义单引号（sh / JS）。
fn sq(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// 转义双引号字符串常量里的内容。
fn dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// 转义 PHP 单引号字符串常量里的内容（单引号串仅 `\'` 与 `\\` 是转义，
/// 其余反斜杠保持字面量，因此 `$` / `\n` 不会被插值或转义）。
fn pq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            c => out.push(c),
        }
    }
    out
}

fn render_curl(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, multipart) = body_parts(spec);
    let mut out = format!("curl -X {method} '{url}'", url = sq(url));
    for (k, v) in headers {
        out.push_str(&format!(" \\\n     -H '{}: {}'", sq(k), sq(v)));
    }
    if let Some(fields) = multipart {
        for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
            let value = match f.value_type {
                MultipartValueType::Text => sq(&f.value),
                MultipartValueType::FilePath => format!("@{}", sq(&f.value)),
            };
            out.push_str(&format!(" \\\n     -F '{}={}'", sq(&f.key), value));
        }
    } else if !body.is_empty() {
        out.push_str(&format!(" \\\n     --data '{}'", sq(&body)));
    }
    if let Some(ct) = content_type {
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            out.push_str(&format!(" \\\n     -H 'Content-Type: {ct}'"));
        }
    }
    out
}

fn render_python(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, multipart) = body_parts(spec);
    let mut out = String::from("import requests\n\n");
    out.push_str(&format!("url = \"{}\"\n", sq(url)));
    out.push_str("headers = {");
    if !headers.is_empty() || content_type.is_some() {
        out.push('\n');
        for (k, v) in headers {
            out.push_str(&format!("    \"{}\": \"{}\",\n", dq(k), dq(v)));
        }
        if let Some(ct) = content_type {
            out.push_str(&format!("    \"Content-Type\": \"{ct}\",\n"));
        }
        out.push_str("}\n");
    } else {
        out.push_str("}\n");
    }
    if let Some(fields) = multipart {
        out.push_str("files = {\n");
        for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
            match f.value_type {
                MultipartValueType::Text => {
                    out.push_str(&format!("    \"{}\": \"{}\",\n", dq(&f.key), dq(&f.value)));
                }
                MultipartValueType::FilePath => {
                    out.push_str(&format!(
                        "    \"{}\": open(\"{}\", \"rb\"),\n",
                        dq(&f.key),
                        dq(&f.value)
                    ));
                }
            }
        }
        out.push_str("}\n");
        out.push_str(&format!(
            "resp = requests.request(\"{method}\", url, headers=headers, files=files)\n"
        ));
    } else if !body.is_empty() {
        out.push_str(&format!("payload = \"{}\"\n", dq(&body)));
        out.push_str(&format!(
            "resp = requests.request(\"{method}\", url, headers=headers, data=payload)\n"
        ));
    } else {
        out.push_str(&format!(
            "resp = requests.request(\"{method}\", url, headers=headers)\n"
        ));
    }
    out.push_str("print(resp.status_code, resp.text)\n");
    out
}

fn render_js(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, multipart) = body_parts(spec);
    let mut out = String::new();
    out.push_str(&format!("const url = '{}';\n", sq(url)));
    out.push_str(&format!("const options = {{\n  method: '{method}',\n"));
    if !headers.is_empty() || content_type.is_some() {
        out.push_str("  headers: {\n");
        for (k, v) in headers {
            out.push_str(&format!("    '{}': '{}',\n", dq(k), dq(v)));
        }
        if let Some(ct) = content_type {
            out.push_str(&format!("    'Content-Type': '{ct}',\n"));
        }
        out.push_str("  },\n");
    }
    if let Some(fields) = multipart {
        out.push_str("const fd = new FormData();\n");
        for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
            match f.value_type {
                MultipartValueType::Text => {
                    out.push_str(&format!("fd.append(\"{}\", \"{}\");\n", dq(&f.key), dq(&f.value)));
                }
                MultipartValueType::FilePath => {
                    out.push_str(&format!(
                        "fd.append(\"{}\", yourFile); // 文件字段：将 yourFile 替换为你的 File 对象\n",
                        dq(&f.key)
                    ));
                }
            }
        }
        out.push_str("  body: fd,\n");
    } else if !body.is_empty() {
        let is_json = matches!(content_type, Some("application/json"));
        if is_json {
            out.push_str(&format!("  body: JSON.stringify({body}),\n"));
        } else {
            out.push_str(&format!("  body: '{}',\n", dq(&body)));
        }
    }
    out.push_str("};\n");
    out.push_str("const resp = await fetch(url, options);\n");
    out.push_str("const data = await resp.text();\n");
    out.push_str("console.log(resp.status, data);\n");
    out
}

fn render_go(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, _) = body_parts(spec);
    let mut out = String::from("package main\n\nimport (\n");
    out.push_str("  \"bytes\"\n  \"fmt\"\n  \"io\"\n  \"net/http\"\n)\n\n");
    out.push_str("func main() {\n");
    if body.is_empty() {
        out.push_str(&format!(
            "  req, err := http.NewRequest(\"{method}\", \"{}\", nil)\n",
            dq(url)
        ));
    } else {
        // Go raw string literal（反引号）无法包含反引号；改用双引号字符串，
        // 对 `"`、`\`、`\n`、`\r`、`\t` 做标准转义。
        out.push_str(&format!(
            "  payload := []byte(\"{}\")\n",
            dq(&body)
        ));
        out.push_str(&format!(
            "  req, err := http.NewRequest(\"{method}\", \"{}\", bytes.NewBuffer(payload))\n",
            dq(url)
        ));
    }
    out.push_str("  if err != nil {\n    panic(err)\n  }\n");
    for (k, v) in headers {
        out.push_str(&format!("  req.Header.Set(\"{}\", \"{}\")\n", dq(k), dq(v)));
    }
    if let Some(ct) = content_type {
        out.push_str(&format!("  req.Header.Set(\"Content-Type\", \"{ct}\")\n"));
    }
    out.push_str("  resp, err := http.DefaultClient.Do(req)\n");
    out.push_str("  if err != nil {\n    fmt.Println(\"请求失败:\", err)\n    return\n  }\n");
    out.push_str("  defer resp.Body.Close()\n");
    out.push_str("  data, _ := io.ReadAll(resp.Body)\n");
    out.push_str("  fmt.Println(resp.Status, string(data))\n");
    out.push_str("}\n");
    out
}

fn render_java(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, multipart) = body_parts(spec);
    let mut out = String::from(
        "import okhttp3.*;\nimport java.io.File;\nimport java.io.IOException;\n\n\
         public class Main {\n  public static void main(String[] args) throws IOException {\n\
         \x20   OkHttpClient client = new OkHttpClient();\n\n",
    );
    let body_expr = body_expr_java(method, &body, content_type, multipart);
    out.push_str(&body_expr);
    out.push_str("    Request request = new Request.Builder()\n");
    out.push_str(&format!("      .url(\"{}\")\n", dq(url)));
    out.push_str(&format!(
        "      .method(\"{}\", body)\n",
        method
    ));
    for (k, v) in headers {
        out.push_str(&format!("      .addHeader(\"{}\", \"{}\")\n", dq(k), dq(v)));
    }
    if let Some(ct) = content_type {
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            out.push_str(&format!("      .addHeader(\"Content-Type\", \"{ct}\")\n"));
        }
    }
    out.push_str("      .build();\n\n");
    out.push_str("    try (Response response = client.newCall(request).execute()) {\n");
    out.push_str("      System.out.println(response.code());\n");
    out.push_str("      System.out.println(response.body() != null ? response.body().string() : \"\");\n");
    out.push_str("    }\n  }\n}\n");
    out
}

/// Java 侧 body 局部变量片段（OkHttp `RequestBody`）。
fn body_expr_java(
    method: &HttpMethod,
    body: &str,
    content_type: Option<&'static str>,
    multipart: Option<&Vec<MultipartField>>,
) -> String {
    if let Some(fields) = multipart {
        let mut out =
            String::from("    MultipartBody.Builder builder = new MultipartBody.Builder()\n");
        out.push_str("      .setType(MultipartBody.FORM)\n");
        for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
            match f.value_type {
                MultipartValueType::Text => out.push_str(&format!(
                    "      .addFormDataPart(\"{}\", \"{}\")\n",
                    dq(&f.key),
                    dq(&f.value)
                )),
                MultipartValueType::FilePath => out.push_str(&format!(
                    "      .addFormDataPart(\"{}\", \"{}\", RequestBody.create(MediaType.parse(\"application/octet-stream\"), new File(\"{}\")))\n",
                    dq(&f.key),
                    file_name(&f.value),
                    dq(&f.value)
                )),
            }
        }
        out.push_str("      .build();\n    RequestBody body = builder;");
        let _ = method;
        return out;
    }
    let ct = content_type.unwrap_or("application/json");
    if body.is_empty() {
        "    RequestBody body = null;".to_string()
    } else {
        format!(
            "    MediaType mediaType = MediaType.parse(\"{ct}\");\n    RequestBody body = RequestBody.create(mediaType, \"{}\");",
            dq(body)
        )
    }
}

/// 从路径提取文件名（Java multipart 的 form 文件名部分）。
fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn render_php(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    spec: &BodySpec,
) -> String {
    let (body, content_type, multipart) = body_parts(spec);
    let mut out = String::from("<?php\n\n$ch = curl_init();\n");
    out.push_str(&format!("curl_setopt($ch, CURLOPT_URL, \"{}\");\n", dq(url)));
    out.push_str(&format!(
        "curl_setopt($ch, CURLOPT_CUSTOMREQUEST, \"{}\");\n",
        method
    ));
    if let Some(fields) = multipart {
        out.push_str("curl_setopt($ch, CURLOPT_POSTFIELDS, array(\n");
        for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
            match f.value_type {
                MultipartValueType::Text => out.push_str(&format!(
                    "    \"{}\" => \"{}\",\n",
                    pq(&f.key),
                    pq(&f.value)
                )),
                MultipartValueType::FilePath => out.push_str(&format!(
                    "    \"{}\" => new CURLFile(\"{}\"),\n",
                    pq(&f.key),
                    pq(&f.value)
                )),
            }
        }
        out.push_str("));\n");
    } else if !body.is_empty() {
        out.push_str(&format!("curl_setopt($ch, CURLOPT_POSTFIELDS, \"{}\");\n", dq(&body)));
    }
    let has_ct = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
    if !headers.is_empty() || (content_type.is_some() && !has_ct) {
        out.push_str("curl_setopt($ch, CURLOPT_HTTPHEADER, array(\n");
        for (k, v) in headers {
            out.push_str(&format!("    \"{}: {}\",\n", dq(k), dq(v)));
        }
        if let Some(ct) = content_type {
            if !has_ct {
                out.push_str(&format!("    \"Content-Type: {ct}\",\n"));
            }
        }
        out.push_str("));\n");
    }
    out.push_str("curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);\n\n");
    out.push_str("$response = curl_exec($ch);\n");
    out.push_str("$err = curl_error($ch);\n");
    out.push_str("curl_close($ch);\n\n");
    out.push_str("if ($err) {\n    echo \"cURL Error #:\" . $err;\n} else {\n    echo $response;\n}\n");
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::HttpMethod;

    #[test]
    fn curl_includes_method_headers_and_auth() {
        let method = HttpMethod::POST;
        let body = BodySpec::Json {
            raw: "{\"name\":\"a\\\"b\"}".into(),
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/users?page=1",
            headers: &[],
            body: &body,
            auth: &AuthSpec::Bearer {
                token: "tok123".into(),
            },
        };
        let code = render(Lang::Curl, &req);
        assert!(code.contains("curl -X POST 'https://api.example.com/users?page=1'"));
        assert!(code.contains("Authorization: Bearer tok123"));
        assert!(code.contains("--data"));
        assert!(code.contains("Content-Type: application/json"));
    }

    #[test]
    fn python_includes_body_and_header() {
        let method = HttpMethod::POST;
        let body = BodySpec::Json { raw: "{}".into() };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/users?page=1",
            headers: &[],
            body: &body,
            auth: &AuthSpec::Bearer {
                token: "tok123".into(),
            },
        };
        let code = render(Lang::Python, &req);
        assert!(code.contains("url = \"https://api.example.com/users?page=1\""));
        assert!(code.contains("Authorization"));
        assert!(code.contains("payload = "));
        assert!(code.contains("requests.request(\"POST\""));
    }

    #[test]
    fn js_json_body_uses_stringify() {
        let method = HttpMethod::POST;
        let body = BodySpec::Json {
            raw: "{\"a\":1}".into(),
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/x",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::JavaScript, &req);
        assert!(code.contains("JSON.stringify("));
        assert!(code.contains("fetch(url, options)"));
    }

    #[test]
    fn go_body_uses_double_quoted_string_with_escaping() {
        // body 含反引号、双引号、换行、反斜杠：必须走双引号字符串并标准转义，
        // 不能再用 Go raw string literal（反引号无法包含反引号）。
        let method = HttpMethod::POST;
        let body = BodySpec::Json {
            raw: "{\"msg\":\"a`b\\n\\\"c\"\r\n}".into(),
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/s",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Go, &req);
        assert!(code.contains("payload := []byte(\""));
        assert!(!code.contains("payload := []byte(`"));
        // 换行 / 回车 / 双引号 / 反斜杠 均被转义为合法 Go 转义序列。
        assert!(code.contains("\\n"));
        assert!(code.contains("\\r"));
        assert!(code.contains("\\\""));
        assert!(code.contains("\\\\"));
        // 反引号保留（双引号字符串内合法）。
        assert!(code.contains("a`b"));
    }

    #[test]
    fn multipart_curl_uses_dash_f() {
        let method = HttpMethod::POST;
        let body = BodySpec::Multipart {
            fields: vec![
                MultipartField {
                    key: "name".into(),
                    value_type: MultipartValueType::Text,
                    value: "张三".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "avatar".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/tmp/a.png".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "skip".into(),
                    value_type: MultipartValueType::Text,
                    value: "x".into(),
                    enabled: false,
                },
            ],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/upload",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Curl, &req);
        assert!(code.contains("-F 'name=张三'"));
        assert!(code.contains("-F 'avatar=@/tmp/a.png'"));
        assert!(!code.contains("skip"));
        assert!(!code.contains("--data"));
    }

    #[test]
    fn multipart_python_uses_files_dict() {
        let method = HttpMethod::POST;
        let body = BodySpec::Multipart {
            fields: vec![
                MultipartField {
                    key: "name".into(),
                    value_type: MultipartValueType::Text,
                    value: "v".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "avatar".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/tmp/a.png".into(),
                    enabled: true,
                },
            ],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/upload",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Python, &req);
        assert!(code.contains("files = {"));
        assert!(code.contains("\"name\": \"v\""));
        assert!(code.contains("\"avatar\": open(\"/tmp/a.png\", \"rb\")"));
        assert!(code.contains("files=files"));
        assert!(!code.contains("data=payload"));
    }

    #[test]
    fn multipart_js_uses_formdata() {
        let method = HttpMethod::POST;
        let body = BodySpec::Multipart {
            fields: vec![
                MultipartField {
                    key: "name".into(),
                    value_type: MultipartValueType::Text,
                    value: "v".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "avatar".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/tmp/a.png".into(),
                    enabled: true,
                },
            ],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/upload",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::JavaScript, &req);
        assert!(code.contains("const fd = new FormData();"));
        assert!(code.contains("fd.append(\"name\", \"v\");"));
        assert!(code.contains("fd.append(\"avatar\", yourFile);"));
        assert!(code.contains("body: fd,"));
        assert!(!code.contains("JSON.stringify"));
    }

    #[test]
    fn java_okhttp_json_body_and_auth() {
        let method = HttpMethod::POST;
        let body = BodySpec::Json {
            raw: "{\"name\":\"a\"}".into(),
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/users",
            headers: &[],
            body: &body,
            auth: &AuthSpec::Bearer {
                token: "tok123".into(),
            },
        };
        let code = render(Lang::Java, &req);
        assert!(code.contains("import okhttp3.*;"));
        assert!(code.contains(".url(\"https://api.example.com/users\")"));
        assert!(code.contains(".method(\"POST\", body)"));
        assert!(code.contains("RequestBody body = RequestBody.create(mediaType, \"{\\\"name\\\":\\\"a\\\"}\");"));
        assert!(code.contains("MediaType.parse(\"application/json\")"));
        assert!(code.contains(".addHeader(\"Authorization\", \"Bearer tok123\")"));
    }

    #[test]
    fn java_okhttp_get_without_body() {
        let method = HttpMethod::GET;
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/g",
            headers: &[],
            body: &BodySpec::None,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Java, &req);
        assert!(code.contains("RequestBody body = null;"));
        assert!(code.contains(".method(\"GET\", body)"));
    }

    #[test]
    fn java_okhttp_multipart() {
        let method = HttpMethod::POST;
        let body = BodySpec::Multipart {
            fields: vec![
                MultipartField {
                    key: "name".into(),
                    value_type: MultipartValueType::Text,
                    value: "v".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "avatar".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/tmp/a.png".into(),
                    enabled: true,
                },
            ],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/u",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Java, &req);
        assert!(code.contains("MultipartBody.Builder builder = new MultipartBody.Builder()"));
        assert!(code.contains("setType(MultipartBody.FORM)"));
        assert!(code.contains(".addFormDataPart(\"name\", \"v\")"));
        assert!(code.contains("new File(\"/tmp/a.png\")"));
        assert!(code.contains(".addFormDataPart(\"avatar\", \"a.png\","));
    }

    #[test]
    fn php_curl_json_body_and_headers() {
        let method = HttpMethod::POST;
        let body = BodySpec::Json {
            raw: "{\"name\":\"a\"}".into(),
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/users",
            headers: &[],
            body: &body,
            auth: &AuthSpec::Bearer {
                token: "tok123".into(),
            },
        };
        let code = render(Lang::Php, &req);
        assert!(code.starts_with("<?php"));
        assert!(code.contains("curl_setopt($ch, CURLOPT_URL, \"https://api.example.com/users\");"));
        assert!(code.contains("CURLOPT_CUSTOMREQUEST, \"POST\""));
        assert!(code.contains("CURLOPT_POSTFIELDS"));
        assert!(code.contains("\"Authorization: Bearer tok123\""));
        assert!(code.contains("\"Content-Type: application/json\""));
        assert!(code.contains("curl_exec($ch)"));
    }

    #[test]
    fn php_curl_multipart_uses_curlfile() {
        let method = HttpMethod::POST;
        let body = BodySpec::Multipart {
            fields: vec![
                MultipartField {
                    key: "name".into(),
                    value_type: MultipartValueType::Text,
                    value: "a'b".into(),
                    enabled: true,
                },
                MultipartField {
                    key: "avatar".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/tmp/a.png".into(),
                    enabled: true,
                },
            ],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/u",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Php, &req);
        assert!(code.contains("\"name\" => \"a\\'b\""));
        assert!(code.contains("\"avatar\" => new CURLFile(\"/tmp/a.png\")"));
        assert!(code.contains("CURLOPT_POSTFIELDS, array("));
    }

    #[test]
    fn go_basic_auth_encodes() {
        let method = HttpMethod::GET;
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/s",
            headers: &[],
            body: &BodySpec::None,
            auth: &AuthSpec::Basic {
                username: "user".into(),
                password: "pass".into(),
            },
        };
        let code = render(Lang::Go, &req);
        assert!(code.contains("http.NewRequest(\"GET\""));
        assert!(code.contains("Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn urlencoded_body_encoded() {
        let method = HttpMethod::POST;
        let body = BodySpec::UrlEncoded {
            fields: vec![KeyValue::new("u", "a b")],
        };
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/login",
            headers: &[],
            body: &body,
            auth: &AuthSpec::None,
        };
        let code = render(Lang::Curl, &req);
        assert!(code.contains("u=a%20b"));
        assert!(code.contains("application/x-www-form-urlencoded"));
    }

    #[test]
    fn apikey_header_injected() {
        let method = HttpMethod::GET;
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/m",
            headers: &[],
            body: &BodySpec::None,
            auth: &AuthSpec::ApiKey {
                key: "X-Key".into(),
                value: "v1".into(),
                location: ApiKeyLocation::Header,
            },
        };
        let code = render(Lang::Python, &req);
        assert!(code.contains("X-Key"));
        assert!(code.contains("v1"));
    }

    #[test]
    fn headers_deduplicated() {
        let method = HttpMethod::GET;
        let headers = vec![KeyValue {
            key: "authorization".into(),
            value: "manual".into(),
            enabled: true,
            description: String::new(),
        }];
        let req = GenRequest {
            method: &method,
            url: "https://api.example.com/d",
            headers: &headers,
            body: &BodySpec::None,
            auth: &AuthSpec::Bearer {
                token: "tok".into(),
            },
        };
        let code = render(Lang::Curl, &req);
        assert_eq!(code.matches("Authorization").count(), 1);
        assert!(code.contains("manual"));
    }
}
