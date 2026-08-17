//! 统一 API 数据模型。
//!
//! 与持久层模型（`fox_core::model`）解耦：任何语言生成器只消费本模块的
//! 中立描述，不感知应用内部的 `RequestSpec` / `Endpoint` 结构。

use std::collections::HashMap;

use fox_core::model::HttpMethod;
use serde::{Deserialize, Serialize};

/// 表单/查询参数键值对（保留语义顺序由调用方以 `Vec` 维持）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

impl KeyValuePair {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        KeyValuePair {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// 请求体。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiBody {
    #[default]
    None,
    /// JSON 文本（原始字符串）。
    Json { raw: String },
    /// 表单字段（multipart / urlencoded，由生成器按其语言习惯选择）。
    #[serde(rename = "formdata")]
    FormData { fields: Vec<KeyValuePair> },
    /// 任意原始文本（text/plain 等）。
    Raw { raw: String },
}

impl ApiBody {
    /// 当前 body 类型的内部标识（用于错误上报与 UI 下拉框）。
    pub fn type_name(&self) -> &'static str {
        match self {
            ApiBody::None => "none",
            ApiBody::Json { .. } => "json",
            ApiBody::FormData { .. } => "formdata",
            ApiBody::Raw { .. } => "raw",
        }
    }
}

/// 认证信息。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthInfo {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
    },
}

/// 统一 API 定义：代码生成引擎的唯一输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiDefinition {
    /// 完整请求地址（含路径；查询参数可放这里或 `query_params`）。
    pub url: String,
    pub method: HttpMethod,
    /// 请求头（key 大小写不敏感合并，后写覆盖）。
    pub headers: HashMap<String, String>,
    /// 查询参数，生成时拼接到 URL 上。
    pub query_params: HashMap<String, String>,
    pub body: ApiBody,
    pub auth: AuthInfo,
}

impl ApiDefinition {
    pub fn new(url: impl Into<String>, method: HttpMethod) -> Self {
        ApiDefinition {
            url: url.into(),
            method,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: ApiBody::None,
            auth: AuthInfo::None,
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: ApiBody) -> Self {
        self.body = body;
        self
    }

    pub fn auth(mut self, auth: AuthInfo) -> Self {
        self.auth = auth;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_body_json_shape() {
        let body = ApiBody::Json {
            raw: "{\"a\":1}".into(),
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["type"], "json");
        assert_eq!(json["raw"], "{\"a\":1}");
        let back: ApiBody = serde_json::from_value(json).unwrap();
        assert_eq!(back, body);
        assert_eq!(ApiBody::None.type_name(), "none");
    }

    #[test]
    fn api_definition_roundtrip() {
        let api = ApiDefinition::new("https://api.example.com/users", HttpMethod::POST)
            .query("page", "1")
            .header("Accept", "application/json")
            .body(ApiBody::FormData {
                fields: vec![KeyValuePair::new("name", "fox")],
            })
            .auth(AuthInfo::Bearer {
                token: "tok".into(),
            });

        let json = serde_json::to_value(&api).unwrap();
        assert_eq!(json["method"], "POST");
        assert_eq!(json["query_params"]["page"], "1");
        assert_eq!(json["body"]["type"], "formdata");
        assert_eq!(json["auth"]["type"], "bearer");

        let back: ApiDefinition = serde_json::from_value(json).unwrap();
        assert_eq!(back, api);
    }
}
