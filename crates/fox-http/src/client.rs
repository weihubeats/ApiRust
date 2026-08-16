//! HTTP 请求构建、发送、响应解析（SPEC §14）。

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine;
use bytes::Bytes;
use futures::StreamExt;
use reqwest::{Client, Method, Response};
use url::Url;

use fox_core::model::{
    ApiKeyLocation, AuthSpec, BodySpec, GraphQLError, GraphQLErrorLocation, GraphQLResponse,
    GraphQLSpec, HttpMethod, KeyValue, MultipartField, MultipartValueType, RequestSpec,
};
use fox_core::variable::{resolve_variables, VariableMap};
use fox_core::AppError;

/// 默认超时（秒）。
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
/// 最大响应体大小（字节）。
pub const MAX_BODY_BYTES: usize = 20 * 1024 * 1024;

/// Cookie 数据。
#[derive(Debug, Clone, PartialEq)]
pub struct CookieData {
    pub name: String,
    pub value: String,
}

/// 响应数据。
#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponseData {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    /// 毫秒（浮点，保留亚毫秒精度）。
    pub duration_ms: f64,
    pub size_bytes: usize,
    pub cookies: Vec<CookieData>,
    pub truncated: bool,
}

impl HttpResponseData {
    /// 响应体按 UTF-8 解码（失败时回退到 lossy）。
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// 响应 Content-Type。
    pub fn content_type(&self) -> String {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }
}

fn reqwest_method(method: HttpMethod) -> Method {
    match method {
        HttpMethod::GET => Method::GET,
        HttpMethod::POST => Method::POST,
        HttpMethod::PUT => Method::PUT,
        HttpMethod::DELETE => Method::DELETE,
        HttpMethod::PATCH => Method::PATCH,
        HttpMethod::HEAD => Method::HEAD,
        HttpMethod::OPTIONS => Method::OPTIONS,
    }
}

/// 将已渲染的 Query 参数拼接到 URL。
fn append_query(url: &mut Url, params: &[KeyValue]) {
    for kv in params {
        if !kv.enabled {
            continue;
        }
        let key = kv.key.trim();
        if key.is_empty() {
            continue;
        }
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair(key, &kv.value);
    }
}

/// 渲染后的请求载荷（body 与 content-type）。
#[derive(Debug)]
enum Payload {
    None,
    Bytes(Vec<u8>, Option<String>),
}

/// 构建 multipart/form-data 请求体。
///
/// Text 字段直接作为文本 part；FilePath 字段异步读取文件内容作为 part。
/// 编码后的 body 由 `Form::into_stream` 收集为字节，content-type 由
/// `Form::boundary` 拼出（含 boundary）。
async fn build_multipart(fields: &[MultipartField]) -> Result<Payload, AppError> {
    let mut form = reqwest::multipart::Form::new();
    for field in fields {
        if !field.enabled {
            continue;
        }
        let key = field.key.trim();
        if key.is_empty() {
            continue;
        }
        match field.value_type {
            MultipartValueType::Text => {
                form = form.text(key.to_string(), field.value.clone());
            }
            MultipartValueType::FilePath => {
                let path = Path::new(&field.value);
                let data = tokio::fs::read(path).await.map_err(|e| {
                    AppError::Validation(format!("读取文件 {} 失败：{e}", path.display()))
                })?;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|n| !n.is_empty())
                    .unwrap_or("upload")
                    .to_string();
                let part = reqwest::multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")
                    .map_err(|e| AppError::Validation(format!("multipart 构建失败：{e}")))?;
                form = form.part(key.to_string(), part);
            }
        }
    }
    let content_type = format!("multipart/form-data; boundary={}", form.boundary());
    let mut body = Vec::new();
    let mut stream = form.into_stream();
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk.map_err(AppError::from_reqwest)?);
    }
    Ok(Payload::Bytes(body, Some(content_type)))
}

async fn build_payload(spec: &RequestSpec) -> Result<Payload, AppError> {
    match &spec.body {
        BodySpec::None => Ok(Payload::None),
        BodySpec::Json { raw } => Ok(Payload::Bytes(
            raw.as_bytes().to_vec(),
            Some("application/json".into()),
        )),
        BodySpec::Text { raw } => Ok(Payload::Bytes(
            raw.as_bytes().to_vec(),
            Some("text/plain".into()),
        )),
        BodySpec::UrlEncoded { fields } => {
            let body: Vec<(String, String)> = fields
                .iter()
                .filter(|kv| kv.enabled)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect();
            let body = serde_urlencoded::to_string(body).unwrap_or_default();
            Ok(Payload::Bytes(
                body.into_bytes(),
                Some("application/x-www-form-urlencoded".into()),
            ))
        }
        BodySpec::Multipart { fields } => build_multipart(fields).await,
        BodySpec::GraphQL { spec } => {
            let body = graphql_request_json(spec, &VariableMap::new())?;
            Ok(Payload::Bytes(
                body.into_bytes(),
                Some("application/json".into()),
            ))
        }
    }
}

/// 构建 GraphQL 请求体 JSON：`{"query":..., "variables":..., "operationName":...}`。
///
/// query / variables / operationName 中的 `{{name}}` 占位符会先经
/// `vars` 插值；variables 为空串或 `{}` 时省略该字段，operationName 为空时省略。
/// 请求始终为 POST + `application/json`（GraphQL over HTTP 约定）。
pub fn graphql_request_json(spec: &GraphQLSpec, vars: &VariableMap) -> Result<String, AppError> {
    let query = resolve_variables(&spec.query, vars);
    let variables = resolve_variables(&spec.variables, vars);
    let operation_name = resolve_variables(&spec.operation_name, vars);

    let mut payload = serde_json::Map::new();
    payload.insert("query".into(), serde_json::Value::String(query));
    let trimmed = variables.trim();
    if !trimmed.is_empty() && trimmed != "{}" {
        // 变量必须是合法 JSON 对象；非法时向用户报错而不是发坏请求。
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) if value.is_object() => {
                payload.insert("variables".into(), value);
            }
            Ok(_) => {
                return Err(AppError::Validation("GraphQL 变量必须是 JSON 对象".into()));
            }
            Err(e) => {
                return Err(AppError::Validation(format!("GraphQL 变量 JSON 无效：{e}")));
            }
        }
    }
    if !operation_name.trim().is_empty() {
        payload.insert(
            "operationName".into(),
            serde_json::Value::String(operation_name),
        );
    }
    serde_json::to_string(&serde_json::Value::Object(payload))
        .map_err(|e| AppError::Validation(format!("GraphQL 请求构建失败：{e}")))
}

/// 解析 GraphQL 响应，区分 `data` 与 `errors`。
///
/// 非 JSON 响应返回错误；JSON 合法但缺少 data / errors 时对应字段为空。
pub fn parse_graphql_response(body: &[u8]) -> Result<GraphQLResponse, AppError> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| AppError::Validation(format!("GraphQL 响应不是合法 JSON：{e}")))?;
    let data = value.get("data").cloned();
    let errors = value
        .get("errors")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let message = entry.get("message")?.as_str()?.to_string();
                    let locations = entry.get("locations").and_then(|l| l.as_array()).map(|ls| {
                        ls.iter()
                            .filter_map(|loc| {
                                Some(GraphQLErrorLocation {
                                    line: loc.get("line")?.as_u64()?,
                                    column: loc.get("column")?.as_u64()?,
                                })
                            })
                            .collect()
                    });
                    let path = entry.get("path").and_then(|p| p.as_array()).cloned();
                    Some(GraphQLError {
                        message,
                        locations,
                        path,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(GraphQLResponse { data, errors })
}

/// 是否已有指定头（不区分大小写）。
fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(k, _)| k.trim().eq_ignore_ascii_case(name))
}

/// 认证字段转 Header / Query。
/// 应用认证到请求头 / 查询参数。
///
/// OAuth2：取 access token（过期自动静默刷新），未授权时返回中文错误。
async fn apply_auth(
    headers: &mut Vec<(String, String)>,
    query: &mut Vec<KeyValue>,
    auth: &AuthSpec,
) -> Result<(), AppError> {
    match auth {
        AuthSpec::None => {}
        AuthSpec::Bearer { token } => {
            if !token.is_empty() {
                headers.push(("Authorization".into(), format!("Bearer {token}")));
            }
        }
        AuthSpec::Basic { username, password } => {
            let raw = format!("{username}:{password}");
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
            headers.push(("Authorization".into(), format!("Basic {encoded}")));
        }
        AuthSpec::ApiKey {
            key,
            value,
            location,
        } => {
            if key.is_empty() {
                return Ok(());
            }
            match location {
                ApiKeyLocation::Header => {
                    headers.push((key.clone(), value.clone()));
                }
                ApiKeyLocation::Query => {
                    query.push(KeyValue::new(key.clone(), value.clone()));
                }
            }
        }
        AuthSpec::OAuth2 { .. } => {
            let access = fox_oauth::access_token_for(auth)
                .await
                .map_err(|e| AppError::OAuth2(e.to_string()))?;
            headers.push(("Authorization".into(), format!("Bearer {access}")));
        }
    }
    Ok(())
}

/// 全局共享的 reqwest::Client。
///
/// `Client` 内部维护连接池与 TLS 会话缓存，按请求新建会重复建连、
/// 重新握手，性能低下；`OnceLock` 保证进程内只构建一次，
/// 所有（含并发）请求安全复用同一实例。
///
/// 重定向策略是 Client 级配置，故按 `RequestSpec::follow_redirects`
/// 维护两个共享实例（跟随 = 默认最多 10 跳；不跟随 = `Policy::none`）。
fn shared_client(follow_redirects: bool) -> Result<&'static Client, AppError> {
    static FOLLOW: OnceLock<Result<Client, String>> = OnceLock::new();
    static NO_FOLLOW: OnceLock<Result<Client, String>> = OnceLock::new();
    let entry = if follow_redirects {
        FOLLOW.get_or_init(build_client)
    } else {
        NO_FOLLOW.get_or_init(|| {
            Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| e.to_string())
        })
    };
    match entry {
        Ok(client) => Ok(client),
        Err(e) => Err(AppError::Validation(format!("HTTP 客户端初始化失败：{e}"))),
    }
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        // 禁用系统代理：本地开发（127.0.0.1 / localhost）不受代理干扰。
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())
}

/// 发送 HTTP 请求。
///
/// - `url` 应为已渲染（含变量替换与路径变量）的完整地址。
/// - `timeout_ms` 为超时毫秒数；None 时使用默认 30 秒。
///
/// 复用 [`shared_client`] 全局连接池；超时按请求设置，各请求互不影响，
/// 并发调用是安全的。
pub async fn send_request(
    method: HttpMethod,
    url: &str,
    spec: &RequestSpec,
    timeout_ms: Option<u64>,
) -> Result<HttpResponseData, AppError> {
    send_request_inner(method, url, spec, timeout_ms, None).await
}

/// 带取消能力的发送（用户点击「取消请求」时返回 [`AppError::Cancelled`]）。
///
/// 取消令牌由调用方持有（如 Tauri Command 层的请求注册表）；等待连接 /
/// 响应 / 读取响应体期间任一时刻取消，都会中止底层 reqwest 请求并返回
/// [`AppError::Cancelled`]。不可取消场景请使用 [`send_request`]。
pub async fn send_request_cancel(
    method: HttpMethod,
    url: &str,
    spec: &RequestSpec,
    timeout_ms: Option<u64>,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<HttpResponseData, AppError> {
    send_request_inner(method, url, spec, timeout_ms, Some(cancel)).await
}

async fn send_request_inner(
    method: HttpMethod,
    url: &str,
    spec: &RequestSpec,
    timeout_ms: Option<u64>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> Result<HttpResponseData, AppError> {
    let timeout_ms = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let client = shared_client(spec.follow_redirects)?;

    let mut url = Url::parse(url).map_err(|e| AppError::Validation(format!("URL 无效：{e}")))?;

    // Query：显式 params + ApiKey(query) 认证。
    let mut query_extra: Vec<KeyValue> = Vec::new();
    let mut headers: Vec<(String, String)> = spec
        .headers
        .iter()
        .filter(|kv| kv.enabled)
        .map(|kv| (kv.key.trim().to_string(), kv.value.clone()))
        .collect();
    apply_auth(&mut headers, &mut query_extra, &spec.auth).await?;
    append_query(&mut url, &spec.params);
    append_query(&mut url, &query_extra);

    let payload = build_payload(spec).await?;
    let mut req = client
        .request(reqwest_method(method), url.clone())
        .timeout(Duration::from_millis(timeout_ms));

    for (k, v) in &headers {
        if k.is_empty() {
            continue;
        }
        req = req.header(k, v);
    }
    // payload 之后不再使用，按值解构直接 move body，避免整包克隆
    if let Payload::Bytes(body, content_type) = payload {
        if let Some(ct) = &content_type {
            if !has_header(&headers, "content-type") {
                req = req.header("content-type", ct.as_str());
            }
        }
        req = req.body(body);
    }

    let start = std::time::Instant::now();
    let resp: Response = match cancel {
        Some(token) => {
            let fut = req.send();
            tokio::select! {
                // 取消分支胜出即丢弃请求 future：底层 hyper 连接随之中止
                //（hyper 官方文档：丢弃返回的 future 是取消在途请求的支持方式）。
                _ = token.cancelled() => {
                    return Err(AppError::Cancelled("request aborted by user".to_string()));
                }
                r = fut => r.map_err(AppError::from_reqwest)?,
            }
        }
        None => req.send().await.map_err(AppError::from_reqwest)?,
    };
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let status = resp.status().as_u16();
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let cookies: Vec<CookieData> = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| {
            let value = v.to_str().unwrap_or_default();
            let first = value.split(';').next()?.trim();
            let (name, value) = first.split_once('=')?;
            Some(CookieData {
                name: name.to_string(),
                value: value.to_string(),
            })
        })
        .collect();

    // 读取响应体，超过 MAX_BODY_BYTES 截断并标记；期间可被取消。
    let mut body: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut chunks = resp.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(AppError::Cancelled("request aborted by user".to_string()));
        }
        let chunk = chunk.map_err(AppError::from_reqwest)?;
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            let remaining = MAX_BODY_BYTES - body.len();
            body.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body.extend_from_slice(&chunk);
    }

    let size_bytes = body.len();
    Ok(HttpResponseData {
        status,
        headers,
        body: Bytes::from(body),
        duration_ms,
        size_bytes,
        cookies,
        truncated,
    })
}

/// 把 reqwest 错误翻译为面向用户的中文提示（DNS / 超时 / TLS / 连接失败）。
///
/// 分类逻辑统一收敛在 [`AppError::classify`]，此处仅取其用户消息。
pub fn describe_http_error(e: &reqwest::Error) -> String {
    AppError::classify(e)
        .map(|err| err.user_message())
        .unwrap_or_else(|| "请求失败：服务端未返回有效响应".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::RequestSpec;

    /// 极简本地 HTTP 服务。
    fn start_server(
        handler: impl Fn(&str, &str) -> (u16, String, String) + Send + 'static,
    ) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 8192];
                let _ = stream.read(&mut buf);
                let request = String::from_utf8_lossy(&buf).to_string();
                let head = request.split("\r\n").next().unwrap_or("").to_string();
                let (status, ctype, body) = handler(&head, &request);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    use std::io::{Read, Write};

    #[tokio::test]
    async fn send_get_with_query() {
        let base = start_server(|head, _| {
            assert!(head.starts_with("GET /echo?a=1&b=hello"));
            (200, "text/plain".to_string(), "ok".to_string())
        });
        let spec = RequestSpec {
            params: vec![KeyValue::new("a", "1"), KeyValue::new("b", "hello"), {
                let mut kv = KeyValue::new("off", "x");
                kv.enabled = false;
                kv
            }],
            ..Default::default()
        };
        let resp = send_request(HttpMethod::GET, &format!("{base}/echo"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_text(), "ok");
        assert_eq!(resp.size_bytes, 2);
    }

    #[tokio::test]
    async fn send_post_json() {
        let base = start_server(|head, request| {
            assert!(head.starts_with("POST /data"));
            assert!(request.contains("content-type: application/json"));
            assert!(request.contains("{\"a\":1}"));
            (
                201,
                "application/json".to_string(),
                "{\"ok\":true}".to_string(),
            )
        });
        let spec = RequestSpec {
            body: BodySpec::Json {
                raw: "{\"a\":1}".into(),
            },
            ..Default::default()
        };
        let resp = send_request(HttpMethod::POST, &format!("{base}/data"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 201);
        assert_eq!(resp.content_type(), "application/json");
    }

    // ---------- OAuth2：Bearer 头 + 过期静默刷新 ----------

    /// 本地 OAuth2 服务：/token 返回新令牌；其余路径断言 Authorization == 期望值。
    fn oauth_server(
        expected_bearer: &'static str,
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let refresh_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = refresh_count.clone();
        let base = start_server(move |head, request| {
            if head.contains("/token") {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert!(head.starts_with("POST /token"), "HEAD: {head}");
                assert!(request.contains("grant_type=refresh_token"));
                assert!(request.contains("refresh_token=rt-1"));
                return (
                    200,
                    "application/json".to_string(),
                    "{\"access_token\":\"refreshed-1\",\"token_type\":\"Bearer\",\
                     \"refresh_token\":\"rt-2\",\"expires_in\":3600}"
                        .to_string(),
                );
            }
            let auth = request
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                .map(|l| l.to_string())
                .unwrap_or_default();
            assert!(
                auth.contains(expected_bearer),
                "期望 Authorization 含 {expected_bearer}，实际：{auth}"
            );
            (200, "text/plain".to_string(), "ok".to_string())
        });
        (base, refresh_count)
    }

    fn oauth_spec(expires_in_secs: i64, refresh_token: Option<&str>) -> RequestSpec {
        RequestSpec {
            auth: AuthSpec::OAuth2 {
                client_id: "e2e-client".into(),
                client_secret: "e2e-secret".into(),
                auth_url: "https://idp.example/authorize".into(),
                token_url: String::new(), // 测试中动态填入
                scope: "read".into(),
                redirect_uri: "http://127.0.0.1:9090/callback".into(),
                token: Some(fox_core::model::OAuth2Token {
                    access_token: "stale-token".into(),
                    token_type: "Bearer".into(),
                    refresh_token: refresh_token.map(String::from),
                    expires_at: chrono::Utc::now() + chrono::Duration::seconds(expires_in_secs),
                }),
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn oauth2_expired_token_auto_refreshes() {
        let (base, counter) = oauth_server("Bearer refreshed-1");
        let mut spec = oauth_spec(-10, Some("rt-1"));
        if let AuthSpec::OAuth2 { token_url, .. } = &mut spec.auth {
            *token_url = format!("{base}/token");
        }
        let resp = send_request(HttpMethod::GET, &format!("{base}/echo"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "过期 token 应触发一次刷新"
        );
        // 刷新后的 token 已入缓存 → 第二次请求不再刷新。
        let resp2 = send_request(HttpMethod::GET, &format!("{base}/echo"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp2.status, 200);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "有效 token 不应重复刷新"
        );
    }

    #[tokio::test]
    async fn oauth2_valid_token_uses_bearer_without_refresh() {
        let (base, counter) = oauth_server("Bearer stale-token");
        let mut spec = oauth_spec(3600, Some("rt-1"));
        if let AuthSpec::OAuth2 { token_url, .. } = &mut spec.auth {
            *token_url = format!("{base}/token");
        }
        let resp = send_request(HttpMethod::GET, &format!("{base}/echo"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "有效 token 不应触发刷新"
        );
    }

    #[tokio::test]
    async fn oauth2_unauthorized_reports_error() {
        let (base, _counter) = oauth_server("Bearer unused");
        let spec = RequestSpec {
            auth: AuthSpec::OAuth2 {
                client_id: "u-client".into(),
                client_secret: "s".into(),
                auth_url: "https://idp.example/authorize".into(),
                token_url: format!("{base}/token"),
                scope: String::new(),
                redirect_uri: String::new(),
                token: None,
            },
            ..Default::default()
        };
        let err = send_request(HttpMethod::GET, &format!("{base}/echo"), &spec, None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::OAuth2(_)),
            "应映射为 OAuth2 错误：{err}"
        );
        assert!(err.user_message().contains("立即授权"));
    }

    #[tokio::test]
    async fn send_basic_auth() {
        let base = start_server(|_, request| {
            let expect = base64::engine::general_purpose::STANDARD.encode("u:p");
            let line = request
                .lines()
                .find(|l| l.to_lowercase().starts_with("authorization"))
                .unwrap_or_default()
                .to_string();
            assert_eq!(
                line.to_lowercase(),
                format!("authorization: basic {}", expect.to_lowercase())
            );
            (200, "text/plain".to_string(), "auth-ok".to_string())
        });
        let spec = RequestSpec {
            auth: AuthSpec::Basic {
                username: "u".into(),
                password: "p".into(),
            },
            ..Default::default()
        };
        let resp = send_request(HttpMethod::GET, &format!("{base}/"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn send_urlencoded() {
        let base = start_server(|_, request| {
            assert!(request.contains("content-type: application/x-www-form-urlencoded"));
            assert!(request.contains("k=v"));
            (200, "text/plain".to_string(), "form".to_string())
        });
        let spec = RequestSpec {
            body: BodySpec::UrlEncoded {
                fields: vec![KeyValue::new("k", "v")],
            },
            ..Default::default()
        };
        let resp = send_request(HttpMethod::PUT, &format!("{base}/form"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn connection_refused_reports_error() {
        let spec = RequestSpec::default();
        let err = send_request(HttpMethod::GET, "http://127.0.0.1:1/", &spec, Some(3000)).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn connection_refused_mapped_to_chinese_hint() {
        let spec = RequestSpec::default();
        let err = send_request(HttpMethod::GET, "http://127.0.0.1:1/", &spec, Some(3000))
            .await
            .unwrap_err();
        match err {
            AppError::Connection(_) => (),
            other => panic!("应映射为 Connection 错误：{other}"),
        }
        assert!(err.user_message().contains("连接失败"));
    }

    #[tokio::test]
    async fn dns_failure_mapped_to_dns_variant() {
        let spec = RequestSpec::default();
        // .invalid 顶级域按 RFC 2606 保证不解析。
        let err = send_request(
            HttpMethod::GET,
            "http://rustfox-nonexistent.invalid/",
            &spec,
            Some(3000),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Dns(_)), "应映射为 DNS 错误：{err}");
        assert!(err.user_message().contains("DNS 解析失败"));
    }

    #[tokio::test]
    async fn invalid_url_mapped_to_validation_variant() {
        let spec = RequestSpec::default();
        let err = send_request(HttpMethod::GET, "not a url", &spec, None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Validation(_)),
            "应映射为 Validation：{err}"
        );
    }

    #[tokio::test]
    async fn timeout_is_applied() {
        // 服务端不响应：读请求后挂起。
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 8192];
                let _ = stream.read(&mut buf);
                // 不写响应，挂起直到客户端超时断开。
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = stream.write_all(b"");
            }
        });
        let spec = RequestSpec::default();
        let start = std::time::Instant::now();
        let err = send_request(
            HttpMethod::GET,
            &format!("http://{addr}/slow"),
            &spec,
            Some(500),
        )
        .await;
        assert!(err.is_err());
        assert!(start.elapsed().as_millis() < 3000);
        if let Err(err) = err {
            assert!(
                matches!(err, AppError::NetworkTimeout(_)),
                "应映射为超时错误：{err}"
            );
            assert!(err.user_message().contains("超时"));
        }
    }

    #[tokio::test]
    async fn body_truncated_at_limit() {
        let big = "x".repeat(MAX_BODY_BYTES + 100);
        let base = start_server(move |_, _| (200, "text/plain".to_string(), big.clone()));
        let spec = RequestSpec::default();
        let resp = send_request(HttpMethod::GET, &format!("{base}/big"), &spec, None)
            .await
            .unwrap();
        assert!(resp.truncated);
        assert_eq!(resp.size_bytes, MAX_BODY_BYTES);
    }

    #[tokio::test]
    async fn concurrent_requests_share_client() {
        let base = start_server(|_, _| (200, "text/plain".to_string(), "ok".to_string()));
        let spec = RequestSpec::default();
        let url = format!("{base}/concurrent");
        let mut tasks = Vec::new();
        for _ in 0..10 {
            let spec = spec.clone();
            let url = url.clone();
            tasks.push(tokio::spawn(async move {
                let resp = send_request(HttpMethod::GET, &url, &spec, None)
                    .await
                    .unwrap();
                assert_eq!(resp.status, 200);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }
    }

    #[tokio::test]
    async fn payload_builds_correctly() {
        let spec = RequestSpec {
            body: BodySpec::UrlEncoded {
                fields: vec![KeyValue::new("a", "1"), KeyValue::new("b", "x y"), {
                    let mut kv = KeyValue::new("off", "z");
                    kv.enabled = false;
                    kv
                }],
            },
            ..Default::default()
        };
        let payload = build_payload(&spec).await.unwrap();
        match payload {
            Payload::Bytes(body, ct) => {
                assert_eq!(ct.as_deref(), Some("application/x-www-form-urlencoded"));
                assert_eq!(String::from_utf8(body).unwrap(), "a=1&b=x+y");
            }
            _ => panic!("期望 UrlEncoded payload"),
        }
    }

    #[test]
    fn graphql_json_builds_post_payload() {
        let spec = GraphQLSpec {
            query: "query Hero($id: ID!) { hero(id: $id) { name } }".into(),
            variables: "{\"id\":\"42\"}".into(),
            operation_name: "Hero".into(),
        };
        let json = graphql_request_json(&spec, &VariableMap::new()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["query"],
            "query Hero($id: ID!) { hero(id: $id) { name } }"
        );
        assert_eq!(parsed["variables"]["id"], "42");
        assert_eq!(parsed["operationName"], "Hero");
    }

    #[test]
    fn graphql_json_omits_empty_variables_and_operation() {
        let spec = GraphQLSpec::default();
        let json = graphql_request_json(&spec, &VariableMap::new()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("variables").is_none());
        assert!(parsed.get("operationName").is_none());
        assert_eq!(parsed["query"], "");
    }

    #[test]
    fn graphql_json_interpolates_variables() {
        let spec = GraphQLSpec {
            query: "query { hero(id: {{id}}) { name } }".into(),
            variables: "{\"id\":\"{{hero_id}}\"}".into(),
            operation_name: String::new(),
        };
        let mut vars = VariableMap::new();
        vars.insert("id".into(), "7".into());
        vars.insert("hero_id".into(), "99".into());
        let json = graphql_request_json(&spec, &vars).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["query"].as_str().unwrap().contains("hero(id: 7)"));
        assert_eq!(parsed["variables"]["id"], "99");
    }

    #[test]
    fn graphql_json_rejects_invalid_variables() {
        let spec = GraphQLSpec {
            query: "query { a }".into(),
            variables: "not-json".into(),
            operation_name: String::new(),
        };
        let err = graphql_request_json(&spec, &VariableMap::new()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn graphql_response_parses_data_and_errors() {
        let raw = br#"{"data":{"hero":{"name":"R2-D2"}},"errors":[{"message":"oops","locations":[{"line":1,"column":3}],"path":["hero","name"]}]}"#;
        let resp = parse_graphql_response(raw).unwrap();
        assert_eq!(resp.data.as_ref().unwrap()["hero"]["name"], "R2-D2");
        assert!(resp.has_errors());
        assert_eq!(resp.errors[0].message, "oops");
        assert_eq!(resp.errors[0].locations.as_ref().unwrap()[0].line, 1);
        assert_eq!(resp.errors[0].path.as_ref().unwrap()[0], "hero");
    }

    #[test]
    fn graphql_response_without_errors_has_empty_list() {
        let raw = br#"{"data":{"ok":true}}"#;
        let resp = parse_graphql_response(raw).unwrap();
        assert!(!resp.has_errors());
        assert_eq!(resp.data.as_ref().unwrap()["ok"], true);
    }

    #[test]
    fn graphql_response_rejects_non_json() {
        let err = parse_graphql_response(b"<html>error</html>").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn multipart_text_payload_has_boundary() {
        let spec = RequestSpec {
            body: BodySpec::Multipart {
                fields: vec![
                    MultipartField {
                        key: "note".into(),
                        value_type: MultipartValueType::Text,
                        value: "hello".into(),
                        enabled: true,
                    },
                    MultipartField {
                        key: "off".into(),
                        value_type: MultipartValueType::Text,
                        value: "skip".into(),
                        enabled: false,
                    },
                ],
            },
            ..Default::default()
        };
        let payload = build_payload(&spec).await.unwrap();
        match payload {
            Payload::Bytes(body, ct) => {
                let ct = ct.unwrap();
                assert!(
                    ct.starts_with("multipart/form-data; boundary="),
                    "意外 content-type：{ct}"
                );
                let body = String::from_utf8(body).unwrap();
                assert!(body.contains("name=\"note\""));
                assert!(body.contains("hello"));
                assert!(!body.contains("skip"), "禁用的字段不应发送");
            }
            _ => panic!("期望 Multipart payload"),
        }
    }

    #[tokio::test]
    async fn send_multipart_with_file_upload() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("fox_http_upload_test.txt");
        tokio::fs::write(&file_path, b"hello multipart file")
            .await
            .unwrap();
        let file_path_str = file_path.to_str().unwrap().to_string();

        let base = start_server(|_, request| {
            assert!(request.contains("content-type: multipart/form-data; boundary="));
            assert!(request.contains("name=\"note\""));
            assert!(request.contains("hello from text"));
            assert!(request.contains("name=\"file\""));
            assert!(request.contains("filename=\"fox_http_upload_test.txt\""));
            assert!(request.contains("hello multipart file"));
            (200, "text/plain".to_string(), "uploaded".to_string())
        });
        let spec = RequestSpec {
            body: BodySpec::Multipart {
                fields: vec![
                    MultipartField {
                        key: "note".into(),
                        value_type: MultipartValueType::Text,
                        value: "hello from text".into(),
                        enabled: true,
                    },
                    MultipartField {
                        key: "file".into(),
                        value_type: MultipartValueType::FilePath,
                        value: file_path_str,
                        enabled: true,
                    },
                ],
            },
            ..Default::default()
        };
        let resp = send_request(HttpMethod::POST, &format!("{base}/upload"), &spec, None)
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_text(), "uploaded");
        tokio::fs::remove_file(&file_path).await.unwrap();
    }

    #[tokio::test]
    async fn multipart_missing_file_reports_error() {
        let spec = RequestSpec {
            body: BodySpec::Multipart {
                fields: vec![MultipartField {
                    key: "file".into(),
                    value_type: MultipartValueType::FilePath,
                    value: "/nonexistent/fox_http_missing.txt".into(),
                    enabled: true,
                }],
            },
            ..Default::default()
        };
        let err = build_payload(&spec).await.unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("读取文件"), "意外提示：{msg}"),
            other => panic!("非 Validation 错误：{other}"),
        }
    }

    #[tokio::test]
    async fn auth_api_key_query_appends() {
        let mut headers = Vec::new();
        let mut query = Vec::new();
        apply_auth(
            &mut headers,
            &mut query,
            &AuthSpec::ApiKey {
                key: "apikey".into(),
                value: "secret".into(),
                location: ApiKeyLocation::Query,
            },
        )
        .await
        .unwrap();
        assert!(headers.is_empty());
        assert_eq!(query.len(), 1);
        assert_eq!(query[0].key, "apikey");
        assert_eq!(query[0].value, "secret");
    }
}
