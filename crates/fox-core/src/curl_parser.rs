//! 极简 cURL 命令解析器：把粘贴的命令转成 RustFox 请求模型。
//!
//! 设计约束：
//! - 只依赖 `shell_words` 做带引号的字段切分，其余全部手写状态机；
//! - 不支持的参数（`-v`/`-k`/`-L`/`-s` 等）一律跳过，绝不报错；
//! - 解析失败只有两种：引号未闭合（shell_words 报错）或缺少 URL。

use crate::error::AppError;
use crate::model::{AuthSpec, BodySpec, HttpMethod, KeyValue};

/// cURL 解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurlParsed {
    /// 请求 URL（完整地址或相对路径）。
    pub url: String,
    /// HTTP 方法（`-X` 优先；否则有 `-d` 时 POST，缺省 GET）。
    pub method: HttpMethod,
    /// 请求头（`-H "Key: Value"`）。
    pub headers: Vec<KeyValue>,
    /// 请求体（`-d` / `--data` / `--data-raw`）。
    pub body: Option<BodySpec>,
    /// 认证（`-u user:pass` → Basic）。
    pub auth: AuthSpec,
}

impl Default for CurlParsed {
    fn default() -> Self {
        CurlParsed {
            url: String::new(),
            method: HttpMethod::GET,
            headers: Vec::new(),
            body: None,
            auth: AuthSpec::None,
        }
    }
}

/// 解析 cURL 命令字符串。
pub fn parse_curl(input: &str) -> Result<CurlParsed, AppError> {
    let words = shell_words::split(input)
        .map_err(|e| AppError::Validation(format!("cURL 命令无法解析（引号未闭合？）：{e}")))?;
    if words.is_empty() {
        return Err(AppError::Validation("cURL 命令为空".into()));
    }

    let mut out = CurlParsed::default();
    // 显式方法：`-X` / `--request`，优先级高于 `-d` 推断。
    let mut explicit: Option<HttpMethod> = None;
    let mut has_data = false;
    let mut data_parts: Vec<String> = Vec::new();

    let mut i = 0;
    while i < words.len() {
        let w = &words[i];

        // 命令本身（curl / curl.exe / /usr/bin/curl 等）跳过。
        if i == 0 && (w == "curl" || w == "curl.exe" || w.ends_with("/curl")) {
            i += 1;
            continue;
        }

        // `--` 之后全部视为位置参数（URL）。
        if w == "--" {
            if out.url.is_empty() {
                if let Some(v) = words.get(i + 1) {
                    out.url = v.clone();
                }
            }
            break;
        }

        // 长选项 `--name=value` 形式（如 `--data='{"a":1}'`）。
        if let Some((name, value)) = w.split_once('=') {
            if name.starts_with("--") {
                if is_value_option(name) {
                    apply_value_option(
                        &mut out,
                        &mut explicit,
                        &mut has_data,
                        &mut data_parts,
                        name,
                        value,
                    );
                }
                // 未知长选项整体忽略。
                i += 1;
                continue;
            }
        }

        match w.as_str() {
            "-X" | "--request" => {
                if let Some(v) = words.get(i + 1) {
                    explicit = parse_method(v);
                    i += 1;
                }
            }
            "-H" | "--header" => {
                if let Some(v) = words.get(i + 1) {
                    push_header(&mut out.headers, v);
                    i += 1;
                }
            }
            "-u" | "--user" => {
                if let Some(v) = words.get(i + 1) {
                    out.auth = AuthSpec::Basic {
                        username: parse_user(v).0,
                        password: parse_user(v).1,
                    };
                    i += 1;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
                if let Some(v) = words.get(i + 1) {
                    data_parts.push(v.clone());
                    has_data = true;
                    i += 1;
                }
            }
            "--url" => {
                if let Some(v) = words.get(i + 1) {
                    if out.url.is_empty() {
                        out.url = v.clone();
                    }
                    i += 1;
                }
            }
            // 不支持的参数（-v / -k / -s / -L / -o ...）忽略：仅跳过本 token。
            other if other.starts_with('-') && other.len() > 1 => {}
            // 第一个非参数 token 即 URL。
            _ => {
                if out.url.is_empty() {
                    out.url = w.clone();
                }
            }
        }
        i += 1;
    }

    if out.url.is_empty() {
        return Err(AppError::Validation(
            "cURL 命令中未找到 URL（例如：curl https://api.example.com/users）".into(),
        ));
    }

    out.method = explicit.unwrap_or(if has_data {
        HttpMethod::POST
    } else {
        HttpMethod::GET
    });
    if has_data {
        out.body = Some(infer_body(&data_parts.join("&")));
    }
    Ok(out)
}

/// 以下长选项消耗一个值。
fn is_value_option(name: &str) -> bool {
    matches!(
        name,
        "--url"
            | "--request"
            | "--header"
            | "--user"
            | "--data"
            | "--data-raw"
            | "--data-binary"
            | "--data-urlencode"
    )
}

fn apply_value_option(
    out: &mut CurlParsed,
    explicit: &mut Option<HttpMethod>,
    has_data: &mut bool,
    data_parts: &mut Vec<String>,
    name: &str,
    value: &str,
) {
    match name {
        "--url" => {
            if out.url.is_empty() {
                out.url = value.to_string();
            }
        }
        "--request" => *explicit = parse_method(value),
        "--header" => push_header(&mut out.headers, value),
        "--user" => {
            let (user, pass) = parse_user(value);
            out.auth = AuthSpec::Basic {
                username: user,
                password: pass,
            };
        }
        "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
            data_parts.push(value.to_string());
            *has_data = true;
        }
        _ => {}
    }
}

fn parse_method(v: &str) -> Option<HttpMethod> {
    v.parse::<HttpMethod>().ok()
}

/// `user:pass` 拆分为 (username, password)；无冒号时 password 为空。
fn parse_user(v: &str) -> (String, String) {
    match v.split_once(':') {
        Some((u, p)) => (u.trim().to_string(), p.to_string()),
        None => (v.trim().to_string(), String::new()),
    }
}

/// `Key: Value` 拆为请求头；无冒号或空键则忽略。
fn push_header(headers: &mut Vec<KeyValue>, v: &str) {
    let Some((key, value)) = v.split_once(':') else {
        return;
    };
    let key = key.trim();
    if key.is_empty() {
        return;
    }
    headers.push(KeyValue::new(key.to_string(), value.trim().to_string()));
}

/// 多个 `-d` 以 `&` 连接（与 curl 语义一致）；`{`/`[` 开头且可解析为 JSON 时推断 JSON。
fn infer_body(data: &str) -> BodySpec {
    let raw = data.to_owned();
    let t = raw.trim_start();
    if (t.starts_with('{') || t.starts_with('[')) && serde_json::from_str::<serde_json::Value>(t).is_ok()
    {
        BodySpec::Json { raw }
    } else {
        BodySpec::Text { raw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_basic(parsed: &CurlParsed) -> Option<(String, String)> {
        match &parsed.auth {
            AuthSpec::Basic { username, password } => {
                Some((username.clone(), password.clone()))
            }
            _ => None,
        }
    }

    /// 基础 GET：`curl https://api.example.com/users`
    #[test]
    fn parse_simple_get() {
        let p = parse_curl("curl https://api.example.com/users").unwrap();
        assert_eq!(p.url, "https://api.example.com/users");
        assert_eq!(p.method, HttpMethod::GET);
        assert!(p.headers.is_empty());
        assert!(p.body.is_none());
        assert_eq!(p.auth, AuthSpec::None);
    }

    /// Header + JSON Body 的 POST
    #[test]
    fn parse_post_with_header_and_json() {
        let p = parse_curl(
            r#"curl -X POST -H "Content-Type: application/json" -d '{"a":1}' https://api.example.com"#,
        )
        .unwrap();
        assert_eq!(p.method, HttpMethod::POST);
        assert_eq!(p.url, "https://api.example.com");
        assert_eq!(p.headers.len(), 1);
        assert_eq!(p.headers[0].key, "Content-Type");
        assert_eq!(p.headers[0].value, "application/json");
        match &p.body {
            Some(BodySpec::Json { raw }) => assert_eq!(raw, "{\"a\":1}"),
            other => panic!("期望 JSON body，实际 {other:?}"),
        }
    }

    /// Basic Auth：curl -u admin:123 https://api.example.com
    #[test]
    fn parse_basic_auth() {
        let p = parse_curl("curl -u admin:123 https://api.example.com").unwrap();
        let (user, pass) = auth_basic(&p).expect("应为 Basic Auth");
        assert_eq!((user.as_str(), pass.as_str()), ("admin", "123"));
        let p2 = parse_curl("curl --user=admin:123 https://api.example.com").unwrap();
        let (user2, pass2) = auth_basic(&p2).expect("应为 Basic Auth");
        assert_eq!((user2.as_str(), pass2.as_str()), ("admin", "123"));
    }

    /// 复杂 shell 转义：单引号套双引号、双引号套单引号。
    #[test]
    fn parse_complex_quoting() {
        let p = parse_curl(
            r#"curl -H 'X-Foo: "bar baz"' -d "it's a test" 'https://api.example.com/echo'"#,
        )
        .unwrap();
        assert_eq!(p.url, "https://api.example.com/echo");
        assert_eq!(p.headers[0].value, "\"bar baz\"");
        match &p.body {
            Some(BodySpec::Text { raw }) => assert_eq!(raw, "it's a test"),
            other => panic!("期望 Text body，实际 {other:?}"),
        }
    }

    /// `-d` 默认推断 POST（无 -X）。
    #[test]
    fn parse_data_defaults_to_post() {
        let p = parse_curl(r#"curl -d 'name=rustfox' https://api.example.com/users"#).unwrap();
        assert_eq!(p.method, HttpMethod::POST);
        match &p.body {
            Some(BodySpec::Text { raw }) => assert_eq!(raw, "name=rustfox"),
            other => panic!("期望 Text body，实际 {other:?}"),
        }
    }

    /// 多个 -d 以 & 连接。
    #[test]
    fn parse_multiple_data_joined_with_ampersand() {
        let p = parse_curl(r#"curl -d a=1 -d b=2 https://api.example.com"#).unwrap();
        match &p.body {
            Some(BodySpec::Text { raw }) => assert_eq!(raw, "a=1&b=2"),
            other => panic!("期望 Text body，实际 {other:?}"),
        }
        assert_eq!(p.method, HttpMethod::POST);
    }

    /// 不支持的参数直接忽略。
    #[test]
    fn parse_unknown_flags_ignored() {
        let p = parse_curl(
            "curl -v -k -s --compressed -L https://api.example.com/secure",
        )
        .unwrap();
        assert_eq!(p.url, "https://api.example.com/secure");
        assert_eq!(p.method, HttpMethod::GET);
    }

    /// `--url` 与 `--url=` 两种写法、重复 --url 取首个。
    #[test]
    fn parse_url_long_option() {
        let p = parse_curl("curl --url https://one.example.com https://two.example.com").unwrap();
        assert_eq!(p.url, "https://one.example.com");
        let p = parse_curl("curl --url=https://eq.example.com").unwrap();
        assert_eq!(p.url, "https://eq.example.com");
    }

    /// 缺少 URL 时报错（不 panic）。
    #[test]
    fn parse_missing_url_errors() {
        let err = parse_curl("curl -X GET -H 'X-A: 1'");
        assert!(err.is_err());
    }

    /// 引号未闭合报验证错误（不 panic）。
    #[test]
    fn parse_unclosed_quote_errors() {
        let err = parse_curl("curl -d 'oops https://api.example.com");
        assert!(err.is_err());
    }

    /// 命令行 curl 前缀（完整路径）也能识别。
    #[test]
    fn parse_with_curl_bin_path() {
        let p = parse_curl("/usr/bin/curl --insecure https://api.example.com/ping").unwrap();
        assert_eq!(p.url, "https://api.example.com/ping");
    }
}