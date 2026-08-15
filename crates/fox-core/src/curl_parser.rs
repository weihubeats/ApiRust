//! 极简 cURL 命令解析器：把粘贴的命令转成 RustFox 请求模型。
//!
//! 设计约束：
//! - 只依赖 `shell_words` 做带引号的字段切分，其余全部手写状态机；
//! - 不支持的参数（`-v`/`-k`/`-L`/`-s` 等）一律跳过，绝不报错；
//! - 解析失败只有两种：引号未闭合（shell_words 报错）或缺少 URL。

use crate::error::AppError;
use crate::model::{AuthSpec, BodySpec, HttpMethod, KeyValue};

/// cURL 解析结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
            // 未知长选项：取值型（如 --cookie ~/a.txt）跳过其值，其余仅跳过自身。
            other if other.starts_with("--") && other.len() > 2 => {
                if words.get(i + 1).is_some() && VALUE_LONG_OPTIONS.contains(&other) {
                    i += 1;
                }
            }
            // 未知短选项：含取值型字符（如 -b、-A，-i/-k 无值）时一并跳过其值。
            other if other.starts_with('-') && other.len() > 1 => {
                if words.get(i + 1).is_some()
                    && other[1..].chars().any(|c| VALUE_SHORT_OPTS.contains(&c))
                {
                    i += 1;
                }
            }
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

/// 额外消耗一个值（空格形式 `--name value`）的常用长选项；其值不能当作 URL。
const VALUE_LONG_OPTIONS: &[&str] = &[
    "--user-agent",
    "--cookie",
    "--referer",
    "--output",
    "--write-out",
    "--proxy",
    "--cookie-jar",
    "--cert",
    "--form",
    "--config",
    "--max-time",
    "--max-redirs",
    "--connect-timeout",
    "--limit-rate",
    "--range",
    "--upload-file",
    "--resolve",
    "--retry",
    "--retry-delay",
    "--interface",
    "--cacert",
    "--capath",
    "--key",
    "--proto",
    "--proto-redir",
];

/// 消耗一个值（后接一个 token）的短选项字符（`-i`/`-s`/`-L`/`-k` 等无值）。
const VALUE_SHORT_OPTS: &[char] = &[
    'A', 'b', 'c', 'd', 'e', 'E', 'F', 'H', 'K', 'm', 'o', 'P', 'r', 'T', 'U', 'u', 'w', 'x', 'X',
    't', 'z',
];

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

/// `{`/`[` 开头且可解析为 JSON 时推断 JSON。
///
/// 启发式分级：先跳过首尾空白检查首尾字符（非 `{`/`[` 开头或
/// 非 `}`/`]` 结尾直接判 Text），再对「可能 JSON 且较短（< 64KB）」
/// 的载荷调用 serde_json 精确验证，避免大 JSON 完整解析的开销。
fn infer_body(data: &str) -> BodySpec {
    let raw = data.to_owned();
    let trimmed = data.trim();
    const MAX_PRECISE_CHECK_BYTES: usize = 64 * 1024;
    let plausible_json = matches!(
        (trimmed.as_bytes().first(), trimmed.as_bytes().last()),
        (Some(b'{' | b'['), Some(b'}' | b']'))
    );
    // 启发式命中（首尾字符像 JSON）且负载较小时再做完整解析验证。
    let is_json = plausible_json
        && trimmed.len() <= MAX_PRECISE_CHECK_BYTES
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok();
    if is_json {
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
            AuthSpec::Basic { username, password } => Some((username.clone(), password.clone())),
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
        let p = parse_curl("curl -v -k -s --compressed -L https://api.example.com/secure").unwrap();
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

    /// 取值型选项（-b/-A/--cookie 等）的值不能误当成 URL。
    #[test]
    fn parse_cookie_flag_value_not_url() {
        let p = parse_curl(
            r#"curl -i --header "Content-Type:application/json" -X GET -b ~/cookie.txt http://www.baidu.com"#,
        )
        .unwrap();
        assert_eq!(p.url, "http://www.baidu.com");
        assert_eq!(p.method, HttpMethod::GET);
        assert_eq!(p.headers.len(), 1);
        assert_eq!(p.headers[0].key, "Content-Type");
        assert_eq!(p.headers[0].value, "application/json");
    }

    /// 长选项空格取值形式（--cookie 等）也跳过值。
    #[test]
    fn parse_long_value_options_skipped() {
        let p = parse_curl(
            r#"curl --cookie "a=1; b=2" --user-agent "Mozilla/5.0" -x http://proxy:8080 https://api.example.com"#,
        )
        .unwrap();
        assert_eq!(p.url, "https://api.example.com");
        assert_eq!(p.method, HttpMethod::GET);
        assert!(p.headers.is_empty());
    }

    /// 组合短选项（-ikLb 等价于 -i -k -L -b）同样跳过 -b 的值。
    #[test]
    fn parse_combined_short_flags_with_value() {
        let p = parse_curl(r#"curl -ikLb "a=1" https://api.example.com"#).unwrap();
        assert_eq!(p.url, "https://api.example.com");
        assert_eq!(p.method, HttpMethod::GET);
    }

    /// 首字符不是 `{`/`[`：启发式直接判 Text，不触发 JSON 解析。
    #[test]
    fn infer_body_non_json_start_is_text() {
        match infer_body("name=rustfox&page=1") {
            BodySpec::Text { .. } => {}
            other => panic!("期望 Text，实际 {other:?}"),
        }
    }

    /// 首尾括号不配对（如截断的 JSON）：判 Text。
    #[test]
    fn infer_body_unmatched_braces_is_text() {
        match infer_body("{\"a\":1") {
            BodySpec::Text { .. } => {}
            other => panic!("期望 Text，实际 {other:?}"),
        }
        match infer_body("[1,2,3}") {
            BodySpec::Text { .. } => {}
            other => panic!("期望 Text，实际 {other:?}"),
        }
    }

    /// 首尾像 JSON 但内容非法：精确解析后回退 Text。
    #[test]
    fn infer_body_invalid_json_is_text() {
        match infer_body("{\"a\":}") {
            BodySpec::Text { .. } => {}
            other => panic!("期望 Text，实际 {other:?}"),
        }
    }

    /// 合法 JSON（含首尾空白）判 Json，且保留原始内容。
    #[test]
    fn infer_body_valid_json_with_whitespace() {
        match infer_body("  {\"a\":1}  ") {
            BodySpec::Json { raw } => assert_eq!(raw, "  {\"a\":1}  "),
            other => panic!("期望 Json，实际 {other:?}"),
        }
    }

    /// 超过 64KB 的载荷跳过精确 JSON 解析，直接判 Text（避免大负载解析开销）。
    #[test]
    fn infer_body_huge_payload_skips_precise_check() {
        let huge = format!("{{\"pad\":\"{}\"}}", "x".repeat(65 * 1024));
        match infer_body(&huge) {
            BodySpec::Text { .. } => {}
            other => panic!("期望 Text（超限跳过解析），实际 {other:?}"),
        }
    }
}
