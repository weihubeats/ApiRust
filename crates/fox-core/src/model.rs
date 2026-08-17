//! 领域模型：与 SPEC 第 8~12 节保持一致。

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// HTTP 方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::GET,
            HttpMethod::POST,
            HttpMethod::PUT,
            HttpMethod::DELETE,
            HttpMethod::PATCH,
            HttpMethod::HEAD,
            HttpMethod::OPTIONS,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HttpMethod {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "PATCH" => Ok(HttpMethod::PATCH),
            "HEAD" => Ok(HttpMethod::HEAD),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            other => Err(AppError::Validation(format!("不支持的 HTTP 方法：{other}"))),
        }
    }
}

/// 接口状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EndpointStatus {
    Designing,
    #[default]
    Developing,
    Testing,
    Released,
    Deprecated,
}

impl EndpointStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EndpointStatus::Designing => "designing",
            EndpointStatus::Developing => "developing",
            EndpointStatus::Testing => "testing",
            EndpointStatus::Released => "released",
            EndpointStatus::Deprecated => "deprecated",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EndpointStatus::Designing => "设计中",
            EndpointStatus::Developing => "开发中",
            EndpointStatus::Testing => "测试中",
            EndpointStatus::Released => "已发布",
            EndpointStatus::Deprecated => "已废弃",
        }
    }
}

/// 键值对：用于 Query / Header / Path 变量等。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}

fn default_true() -> bool {
    true
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: String::new(),
        }
    }
}

/// API Key 放置位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

/// 认证方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthSpec {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    #[serde(rename = "apikey")]
    ApiKey {
        key: String,
        value: String,
        #[serde(rename = "in")]
        location: ApiKeyLocation,
    },
    /// OAuth2 授权码流（Authorization Code Grant）。
    ///
    /// 浏览器跳转 `auth_url` 完成授权 → 本地回调服务器（127.0.0.1:9090）收到 code →
    /// `token_url` 换取 access_token / refresh_token → 存入 `token`。
    /// 过期后由 `fox-oauth` 用 refresh_token 静默刷新。
    #[serde(rename = "oauth2")]
    OAuth2 {
        client_id: String,
        client_secret: String,
        /// 授权页地址（GET，追加 response_type=code / state / redirect_uri / scope）。
        auth_url: String,
        /// 换取与刷新 token 的地址（POST，form 编码）。
        token_url: String,
        scope: String,
        /// 回调地址，须与授权服务注册的一致（默认 http://127.0.0.1:9090/callback）。
        redirect_uri: String,
        /// 已获取的令牌；授权成功后写入，刷新后由缓存回填。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<OAuth2Token>,
    },
}

/// OAuth2 令牌（access + refresh + 过期时刻）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2Token {
    pub access_token: String,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// 过期时刻（UTC）。
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl OAuth2Token {
    /// 已过期（`expires_at` 早于当前时间）。
    pub fn is_expired(&self) -> bool {
        self.expires_at <= chrono::Utc::now()
    }

    /// 距过期不足 `threshold` 视为即将过期（用于提前静默刷新）。
    pub fn expires_within(&self, threshold: chrono::Duration) -> bool {
        self.expires_at - chrono::Utc::now() <= threshold
    }
}

/// OAuth2 授权状态（供 UI 状态指示器）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuth2Status {
    /// 未授权（无 token 或无 refresh_token）。
    Unauthorized,
    /// 已授权，token 有效且未临近过期。
    Valid,
    /// 即将过期（≤ 60 秒），请求时将自动刷新。
    ExpiringSoon,
    /// 已过期（但仍有 refresh_token，请求时将自动刷新）。
    Expired,
}

impl AuthSpec {
    /// 若为 OAuth2，返回其授权状态。
    pub fn oauth2_status(&self) -> Option<OAuth2Status> {
        let AuthSpec::OAuth2 { token, .. } = self else {
            return None;
        };
        let Some(token) = token else {
            return Some(OAuth2Status::Unauthorized);
        };
        if token.is_expired() {
            return Some(OAuth2Status::Expired);
        }
        if token.expires_within(chrono::Duration::seconds(60)) {
            return Some(OAuth2Status::ExpiringSoon);
        }
        Some(OAuth2Status::Valid)
    }
}

/// Multipart 值类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultipartValueType {
    Text,
    FilePath,
}

/// Multipart 字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartField {
    pub key: String,
    pub value_type: MultipartValueType,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// GraphQL 请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GraphQLSpec {
    pub query: String,
    /// 变量 JSON 文本（空串或 "{}" 表示无变量）。
    pub variables: String,
    pub operation_name: String,
}

/// GraphQL 错误位置（errors[].locations[]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLErrorLocation {
    pub line: u64,
    pub column: u64,
}

/// GraphQL 错误条目（errors[]）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<GraphQLErrorLocation>>,
    pub path: Option<Vec<serde_json::Value>>,
}

/// 解析后的 GraphQL 响应：`data` 与 `errors` 并存或互斥。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GraphQLError>,
}

impl GraphQLResponse {
    /// 是否存在错误。
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// 请求 Body。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BodySpec {
    #[default]
    None,
    Json {
        raw: String,
    },
    Text {
        raw: String,
    },
    #[serde(rename = "urlencoded")]
    UrlEncoded {
        fields: Vec<KeyValue>,
    },
    Multipart {
        fields: Vec<MultipartField>,
    },
    GraphQL {
        spec: GraphQLSpec,
    },
    /// 二进制文件请求体：发送时由执行器读取 `path` 指向文件的原始字节，
    /// Content-Type 默认 application/octet-stream（用户显式设置的请求头优先）。
    Binary {
        path: String,
    },
}

impl BodySpec {
    /// 当前 body 模式的内部名称（用于 UI 下拉框）。
    pub fn mode_name(&self) -> &'static str {
        match self {
            BodySpec::None => "none",
            BodySpec::Json { .. } => "json",
            BodySpec::Text { .. } => "text",
            BodySpec::UrlEncoded { .. } => "urlencoded",
            BodySpec::Multipart { .. } => "multipart",
            BodySpec::GraphQL { .. } => "graphql",
            BodySpec::Binary { .. } => "binary",
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, BodySpec::None)
    }
}

/// 测试配置（pre_request / extract / assertions），存储为 JSON。
pub type TestConfig = serde_json::Value;

/// 统一请求结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestSpec {
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub path_variables: Vec<KeyValue>,
    #[serde(default)]
    pub auth: AuthSpec,
    #[serde(default)]
    pub body: BodySpec,
    /// 编辑器配置 Tab（params/auth/headers/body/...）：用户最近一次停留位置。
    /// 为空时前端按 HTTP Method 智能默认（POST 系 → body，其余 → params）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_tab: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default)]
    pub tests: Option<TestConfig>,
}

fn default_timeout() -> u64 {
    30_000
}

impl Default for RequestSpec {
    fn default() -> Self {
        RequestSpec {
            params: Vec::new(),
            headers: Vec::new(),
            path_variables: Vec::new(),
            auth: AuthSpec::None,
            body: BodySpec::None,
            active_tab: None,
            timeout_ms: 30_000,
            follow_redirects: true,
            tests: None,
        }
    }
}

/// 项目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub variables: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 文件夹。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub project_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 接口。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: Uuid,
    pub project_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub description: String,
    pub status: EndpointStatus,
    pub sort_order: i64,
    pub request: RequestSpec,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 环境。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub variables: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 响应示例。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseExample {
    pub id: Uuid,
    pub endpoint_id: Uuid,
    pub name: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 请求用例（请求快照）。
///
/// 保存接口某次请求的完整 `RequestSpec`（参数 / 认证 / 请求头 / Body / 超时等），
/// 可一键回填编辑器复用，避免反复手工拼装。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestExample {
    pub id: Uuid,
    pub endpoint_id: Uuid,
    pub name: String,
    pub request: RequestSpec,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 测试用例运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TestCaseStatus {
    Success,
    Failed,
    Untested,
}

impl TestCaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestCaseStatus::Success => "Success",
            TestCaseStatus::Failed => "Failed",
            TestCaseStatus::Untested => "Untested",
        }
    }

    pub fn parse(s: &str) -> Option<TestCaseStatus> {
        match s {
            "Success" => Some(TestCaseStatus::Success),
            "Failed" => Some(TestCaseStatus::Failed),
            "Untested" => Some(TestCaseStatus::Untested),
            _ => None,
        }
    }
}

/// 测试用例（Apifox 风格用例管理）。
///
/// 每个用例保存一次调用的独立快照：URL 路径 + Query 参数 + 请求头 + Body，
/// 可单独运行、一键回填调试页重新调用。`request_id` 关联主接口（`endpoints`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    /// 关联的主接口 ID（endpoints.id）。
    pub request_id: Uuid,
    pub name: String,
    /// 用例分组：正向 / 负向 / 边界值 / 安全性 / 其他。
    pub category: String,
    pub method: HttpMethod,
    pub url_path: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    /// body 类型标识：json / form-data / raw / urlencoded / graphql / binary / none。
    pub body_type: String,
    pub body_content: String,
    pub last_run_status: TestCaseStatus,
    pub created_at: DateTime<Utc>,
}

/// Mock 匹配条件项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockMatchItem {
    pub key: String,
    pub value: String,
}

/// 自定义 Mock 规则。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockRule {
    pub id: Uuid,
    pub project_id: Uuid,
    pub endpoint_id: Option<Uuid>,
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub match_query: Vec<MockMatchItem>,
    pub match_headers: Vec<MockMatchItem>,
    pub response_status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body_template: String,
    pub delay_ms: u64,
    pub enabled: bool,
    pub priority: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 单次测试运行结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Option<Uuid>,
    pub name: String,
    pub result_json: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// 请求历史。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestHistory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub endpoint_id: Option<Uuid>,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub request_summary_json: String,
    pub response_summary_json: String,
    pub created_at: DateTime<Utc>,
}

/// WebSocket 待发消息类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
}

impl WsMessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WsMessageType::Text => "text",
            WsMessageType::Binary => "binary",
            WsMessageType::Ping => "ping",
        }
    }
}

/// 持久化的 WebSocket 待发消息（断线 / 队列溢出时落库）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsMessageRecord {
    pub id: Uuid,
    pub message_type: WsMessageType,
    /// Text 为原文；Binary / Ping 为 base64 编码。
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_method_roundtrip() {
        for m in HttpMethod::all() {
            let s = serde_json::to_string(m).unwrap();
            assert_eq!(s, format!("\"{}\"", m.as_str()));
            let back: HttpMethod = serde_json::from_str(&s).unwrap();
            assert_eq!(back, *m);
            let parsed: HttpMethod = m.as_str().parse().unwrap();
            assert_eq!(parsed, *m);
        }
        assert!("TRACE".parse::<HttpMethod>().is_err());
    }

    #[test]
    fn status_serde() {
        let s = serde_json::to_string(&EndpointStatus::Developing).unwrap();
        assert_eq!(s, "\"developing\"");
        let back: EndpointStatus = serde_json::from_str("\"released\"").unwrap();
        assert_eq!(back, EndpointStatus::Released);
    }

    #[test]
    fn request_spec_default_json_shape() {
        let spec = RequestSpec::default();
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["params"], serde_json::json!([]));
        assert_eq!(json["auth"]["type"], "none");
        assert_eq!(json["body"]["mode"], "none");
        assert_eq!(json["timeout_ms"], 30_000);
        assert_eq!(json["follow_redirects"], true);
    }

    #[test]
    fn body_spec_binary_json_shape() {
        let body = BodySpec::Binary {
            path: "/tmp/a.png".into(),
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["mode"], "binary");
        assert_eq!(json["path"], "/tmp/a.png");
        let back: BodySpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn auth_bearer_json_shape() {
        let auth = AuthSpec::Bearer {
            token: "{{token}}".into(),
        };
        let json = serde_json::to_value(&auth).unwrap();
        assert_eq!(json["type"], "bearer");
        assert_eq!(json["token"], "{{token}}");
        let back: AuthSpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, auth);
    }

    #[test]
    fn auth_apikey_json_shape() {
        let auth = AuthSpec::ApiKey {
            key: "X-API-KEY".into(),
            value: "{{api_key}}".into(),
            location: ApiKeyLocation::Header,
        };
        let json = serde_json::to_value(&auth).unwrap();
        assert_eq!(json["type"], "apikey");
        assert_eq!(json["in"], "header");
        let back: AuthSpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, auth);
    }

    #[test]
    fn body_urlencoded_json_shape() {
        let body = BodySpec::UrlEncoded {
            fields: vec![KeyValue::new("a", "1")],
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["mode"], "urlencoded");
        assert_eq!(json["fields"][0]["key"], "a");
        let back: BodySpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn oauth2_json_shape_and_roundtrip() {
        let auth = AuthSpec::OAuth2 {
            client_id: "my-client".into(),
            client_secret: "s3cret".into(),
            auth_url: "https://idp.example.com/authorize".into(),
            token_url: "https://idp.example.com/token".into(),
            scope: "openid profile".into(),
            redirect_uri: "http://127.0.0.1:9090/callback".into(),
            token: Some(OAuth2Token {
                access_token: "at-1".into(),
                token_type: "Bearer".into(),
                refresh_token: Some("rt-1".into()),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            }),
        };
        let json = serde_json::to_value(&auth).unwrap();
        assert_eq!(json["type"], "oauth2");
        assert_eq!(json["client_id"], "my-client");
        assert_eq!(json["token"]["access_token"], "at-1");
        let back: AuthSpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, auth);
    }

    #[test]
    fn oauth2_without_token_serializes_compact() {
        let auth = AuthSpec::OAuth2 {
            client_id: "c".into(),
            client_secret: "s".into(),
            auth_url: String::new(),
            token_url: String::new(),
            scope: String::new(),
            redirect_uri: String::new(),
            token: None,
        };
        let json = serde_json::to_value(&auth).unwrap();
        assert!(json.get("token").is_none());
    }

    #[test]
    fn oauth2_token_status_transitions() {
        let make = |hours: i64| AuthSpec::OAuth2 {
            client_id: String::new(),
            client_secret: String::new(),
            auth_url: String::new(),
            token_url: String::new(),
            scope: String::new(),
            redirect_uri: String::new(),
            token: Some(OAuth2Token {
                access_token: String::new(),
                token_type: String::new(),
                refresh_token: None,
                expires_at: chrono::Utc::now() + chrono::Duration::hours(hours),
            }),
        };
        assert_eq!(make(1).oauth2_status(), Some(OAuth2Status::Valid));
        // 30 秒后过期 → 即将过期
        let soon = AuthSpec::OAuth2 {
            client_id: String::new(),
            client_secret: String::new(),
            auth_url: String::new(),
            token_url: String::new(),
            scope: String::new(),
            redirect_uri: String::new(),
            token: Some(OAuth2Token {
                access_token: String::new(),
                token_type: String::new(),
                refresh_token: None,
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(30),
            }),
        };
        assert_eq!(soon.oauth2_status(), Some(OAuth2Status::ExpiringSoon));
        // 已过期
        let expired = AuthSpec::OAuth2 {
            client_id: String::new(),
            client_secret: String::new(),
            auth_url: String::new(),
            token_url: String::new(),
            scope: String::new(),
            redirect_uri: String::new(),
            token: Some(OAuth2Token {
                access_token: String::new(),
                token_type: String::new(),
                refresh_token: None,
                expires_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            }),
        };
        assert_eq!(expired.oauth2_status(), Some(OAuth2Status::Expired));
        // 无 token
        let none = AuthSpec::OAuth2 {
            client_id: String::new(),
            client_secret: String::new(),
            auth_url: String::new(),
            token_url: String::new(),
            scope: String::new(),
            redirect_uri: String::new(),
            token: None,
        };
        assert_eq!(none.oauth2_status(), Some(OAuth2Status::Unauthorized));
        assert_eq!(AuthSpec::None.oauth2_status(), None);
    }
}
