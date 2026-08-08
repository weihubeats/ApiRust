//! HTTP 请求构建、发送、响应解析（SPEC §14）。

use std::time::Duration;

use base64::Engine;
use bytes::Bytes;
use reqwest::{Client, Method, Response};
use url::Url;

use fox_core::model::{ApiKeyLocation, AuthSpec, BodySpec, HttpMethod, KeyValue, RequestSpec};
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
    pub duration_ms: u64,
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
enum Payload {
    None,
    Bytes(Vec<u8>, Option<&'static str>),
}

fn build_payload(spec: &RequestSpec) -> Payload {
    match &spec.body {
        BodySpec::None => Payload::None,
        BodySpec::Json { raw } => Payload::Bytes(raw.as_bytes().to_vec(), Some("application/json")),
        BodySpec::Text { raw } => Payload::Bytes(raw.as_bytes().to_vec(), Some("text/plain")),
        BodySpec::UrlEncoded { fields } => {
            let body: Vec<(String, String)> = fields
                .iter()
                .filter(|kv| kv.enabled)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect();
            let body = serde_urlencoded::to_string(body).unwrap_or_default();
            Payload::Bytes(body.into_bytes(), Some("application/x-www-form-urlencoded"))
        }
        BodySpec::Multipart { .. } => {
            // Multipart 暂未实现，退回无 body。
            Payload::None
        }
    }
}

/// 是否已有指定头（不区分大小写）。
fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(k, _)| k.trim().eq_ignore_ascii_case(name))
}

/// 认证字段转 Header / Query。
fn apply_auth(headers: &mut Vec<(String, String)>, query: &mut Vec<KeyValue>, auth: &AuthSpec) {
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
                return;
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
    }
}

/// 发送 HTTP 请求。
///
/// - `url` 应为已渲染（含变量替换与路径变量）的完整地址。
/// - `timeout_ms` 为超时毫秒数；None 时使用默认 30 秒。
pub async fn send_request(
    method: HttpMethod,
    url: &str,
    spec: &RequestSpec,
    timeout_ms: Option<u64>,
) -> Result<HttpResponseData, AppError> {
    let timeout_ms = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let client = Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(AppError::Http)?;

    let mut url = Url::parse(url).map_err(|e| AppError::Validation(format!("URL 无效：{e}")))?;

    // Query：显式 params + ApiKey(query) 认证。
    let mut query_extra: Vec<KeyValue> = Vec::new();
    let mut headers: Vec<(String, String)> = spec
        .headers
        .iter()
        .filter(|kv| kv.enabled)
        .map(|kv| (kv.key.trim().to_string(), kv.value.clone()))
        .collect();
    apply_auth(&mut headers, &mut query_extra, &spec.auth);
    append_query(&mut url, &spec.params);
    append_query(&mut url, &query_extra);

    let payload = build_payload(spec);
    let mut req = client.request(reqwest_method(method), url.clone());

    for (k, v) in &headers {
        if k.is_empty() {
            continue;
        }
        req = req.header(k, v);
    }
    if let Payload::Bytes(body, content_type) = &payload {
        if let Some(ct) = content_type {
            if !has_header(&headers, "content-type") {
                req = req.header("content-type", *ct);
            }
        }
        req = req.body(body.clone());
    }

    let start = std::time::Instant::now();
    let resp: Response = req.send().await.map_err(AppError::Http)?;
    let duration_ms = start.elapsed().as_millis() as u64;

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

    // 读取响应体，超过 MAX_BODY_BYTES 截断并标记。
    let mut body: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut chunks = resp.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(AppError::Http)?;
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
    async fn invalid_url_reports_error() {
        let spec = RequestSpec::default();
        let err = send_request(HttpMethod::GET, "not a url", &spec, None).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn connection_refused_reports_error() {
        let spec = RequestSpec::default();
        let err = send_request(HttpMethod::GET, "http://127.0.0.1:1/", &spec, Some(3000)).await;
        assert!(err.is_err());
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

    #[test]
    fn payload_builds_correctly() {
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
        let payload = build_payload(&spec);
        match payload {
            Payload::Bytes(body, ct) => {
                assert_eq!(ct, Some("application/x-www-form-urlencoded"));
                assert_eq!(String::from_utf8(body).unwrap(), "a=1&b=x+y");
            }
            _ => panic!("期望 UrlEncoded payload"),
        }
    }

    #[test]
    fn auth_api_key_query_appends() {
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
        );
        assert!(headers.is_empty());
        assert_eq!(query.len(), 1);
        assert_eq!(query[0].key, "apikey");
        assert_eq!(query[0].value, "secret");
    }
}
