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
}
