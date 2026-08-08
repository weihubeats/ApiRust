//! 客户端代码生成（M13）。
//!
//! 支持 curl / Python (requests) / JavaScript (fetch) / Go (net/http)。
//! URL 传入时即为渲染后的完整地址（含变量与环境替换）。

use base64::Engine;
use fox_core::model::{ApiKeyLocation, AuthSpec, BodySpec, HttpMethod, KeyValue};

/// 目标语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Curl,
    Python,
    JavaScript,
    Go,
}

impl Lang {
    pub fn label(&self) -> &'static str {
        match self {
            Lang::Curl => "curl",
            Lang::Python => "Python (requests)",
            Lang::JavaScript => "JavaScript (fetch)",
            Lang::Go => "Go (net/http)",
        }
    }

    pub fn from_str_cn(s: &str) -> Option<Self> {
        match s {
            "curl" => Some(Lang::Curl),
            "python" => Some(Lang::Python),
            "js" => Some(Lang::JavaScript),
            "go" => Some(Lang::Go),
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
    let (body, content_type) = body_parts(req.body);
    let m = req.method;
    let u = req.url;
    match lang {
        Lang::Curl => render_curl(m, u, &merged, &body, &content_type),
        Lang::Python => render_python(m, u, &merged, &body, &content_type),
        Lang::JavaScript => render_js(m, u, &merged, &body, &content_type),
        Lang::Go => render_go(m, u, &merged, &body, &content_type),
    }
}

/// (body 文本, 内容类型)
fn body_parts(body: &BodySpec) -> (String, Option<&'static str>) {
    match body {
        BodySpec::None => (String::new(), None),
        BodySpec::Json { raw } => (raw.clone(), Some("application/json")),
        BodySpec::Text { raw } => (raw.clone(), None),
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
            (parts.join("&"), Some("application/x-www-form-urlencoded"))
        }
        BodySpec::Multipart { .. } => (String::new(), None),
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

fn render_curl(
    method: &HttpMethod,
    url: &str,
    headers: &[(String, String)],
    body: &str,
    content_type: &Option<&'static str>,
) -> String {
    let mut out = format!("curl -X {method} '{url}'", url = sq(url));
    for (k, v) in headers {
        out.push_str(&format!(" \\\n     -H '{}: {}'", sq(k), sq(v)));
    }
    if !body.is_empty() {
        out.push_str(&format!(" \\\n     --data '{}'", sq(body)));
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
    body: &str,
    content_type: &Option<&'static str>,
) -> String {
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
    if !body.is_empty() {
        out.push_str(&format!("payload = \"{}\"\n", dq(body)));
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
    body: &str,
    content_type: &Option<&'static str>,
) -> String {
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
    if !body.is_empty() {
        let is_json = matches!(content_type, Some(ct) if *ct == "application/json");
        if is_json {
            out.push_str(&format!("  body: JSON.stringify({body}),\n"));
        } else {
            out.push_str(&format!("  body: '{}',\n", dq(body)));
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
    body: &str,
    content_type: &Option<&'static str>,
) -> String {
    let mut out = String::from("package main\n\nimport (\n");
    out.push_str("  \"bytes\"\n  \"fmt\"\n  \"io\"\n  \"net/http\"\n)\n\n");
    out.push_str("func main() {\n");
    if body.is_empty() {
        out.push_str(&format!(
            "  req, err := http.NewRequest(\"{method}\", \"{}\", nil)\n",
            dq(url)
        ));
    } else {
        out.push_str(&format!(
            "  payload := []byte(`{}`)\n",
            body.replace('`', "\u{60}\u{60}")
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
