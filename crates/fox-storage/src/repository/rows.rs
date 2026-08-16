//! 行映射与共享工具（内部）。

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use fox_core::model::*;
use fox_core::{AppError, Result};

#[derive(sqlx::FromRow)]
pub(crate) struct ProjectRow {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) variables_json: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl ProjectRow {
    pub(crate) fn from_model(model: &Project) -> Self {
        ProjectRow {
            id: model.id.to_string(),
            name: model.name.clone(),
            description: model.description.clone(),
            variables_json: serde_json::to_string(&model.variables).unwrap_or_else(|_| "{}".into()),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<Project> {
        Ok(Project {
            id: parse_uuid(&self.id)?,
            name: self.name,
            description: self.description,
            variables: serde_json::from_str(&self.variables_json).unwrap_or_default(),
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct FolderRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) name: String,
    pub(crate) sort_order: i64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl FolderRow {
    pub(crate) fn from_model(model: &Folder) -> FolderRow {
        FolderRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            parent_id: model.parent_id.map(|v| v.to_string()),
            name: model.name.clone(),
            sort_order: model.sort_order,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<Folder> {
        Ok(Folder {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            parent_id: self.parent_id.map(|s| parse_uuid(&s)).transpose()?,
            name: self.name,
            sort_order: self.sort_order,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct EndpointRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) folder_id: Option<String>,
    pub(crate) name: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) sort_order: i64,
    pub(crate) request_json: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl EndpointRow {
    pub(crate) fn from_model(model: &Endpoint) -> EndpointRow {
        EndpointRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            folder_id: model.folder_id.map(|v| v.to_string()),
            name: model.name.clone(),
            method: model.method.as_str().to_string(),
            path: model.path.clone(),
            description: model.description.clone(),
            status: model.status.as_str().to_string(),
            sort_order: model.sort_order,
            request_json: serde_json::to_string(&model.request).unwrap_or_else(|_| "{}".into()),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<Endpoint> {
        Ok(Endpoint {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            folder_id: self.folder_id.map(|s| parse_uuid(&s)).transpose()?,
            name: self.name,
            method: self.method.parse()?,
            path: self.path,
            description: self.description,
            status: match self.status.as_str() {
                "designing" => EndpointStatus::Designing,
                "developing" => EndpointStatus::Developing,
                "testing" => EndpointStatus::Testing,
                "released" => EndpointStatus::Released,
                "deprecated" => EndpointStatus::Deprecated,
                _ => EndpointStatus::Developing,
            },
            sort_order: self.sort_order,
            request: serde_json::from_str(&self.request_json).unwrap_or_default(),
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct EnvironmentRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) name: String,
    pub(crate) variables_json: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl EnvironmentRow {
    pub(crate) fn from_model(model: &Environment) -> EnvironmentRow {
        EnvironmentRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            name: model.name.clone(),
            // M11：变量整体加密后落库（密钥不可用时降级明文，保证可用性）。
            variables_json: encrypt_env_json(&model.variables),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<Environment> {
        Ok(Environment {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            name: self.name,
            variables: decrypt_env_json(&self.variables_json)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct TestRunRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) environment_id: Option<String>,
    pub(crate) name: String,
    pub(crate) result_json: String,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
}

impl TestRunRow {
    pub(crate) fn from_model(model: &TestRun) -> Self {
        TestRunRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            environment_id: model.environment_id.map(|e| e.to_string()),
            name: model.name.clone(),
            result_json: model.result_json.clone(),
            started_at: model.started_at.to_rfc3339(),
            finished_at: model.finished_at.map(|d| d.to_rfc3339()),
        }
    }

    pub(crate) fn into_model(self) -> Result<TestRun> {
        Ok(TestRun {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            environment_id: self.environment_id.as_deref().map(parse_uuid).transpose()?,
            name: self.name,
            result_json: self.result_json,
            started_at: parse_time(&self.started_at)?,
            finished_at: self.finished_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct HistoryRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) endpoint_id: Option<String>,
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) status: Option<i64>,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) request_summary_json: String,
    pub(crate) response_summary_json: String,
    pub(crate) created_at: String,
}

impl HistoryRow {
    pub(crate) fn from_model(model: &RequestHistory) -> Self {
        HistoryRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            endpoint_id: model.endpoint_id.map(|e| e.to_string()),
            method: model.method.clone(),
            url: model.url.clone(),
            status: model.status.map(|s| s as i64),
            duration_ms: model.duration_ms.map(|d| d as i64),
            request_summary_json: model.request_summary_json.clone(),
            response_summary_json: model.response_summary_json.clone(),
            created_at: model.created_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<RequestHistory> {
        Ok(RequestHistory {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            endpoint_id: match self.endpoint_id {
                Some(v) => Some(parse_uuid(&v)?),
                None => None,
            },
            method: self.method,
            url: self.url,
            status: self.status.map(|v| v as u16),
            duration_ms: self.duration_ms.map(|d| d as u64),
            request_summary_json: self.request_summary_json,
            response_summary_json: self.response_summary_json,
            created_at: parse_time(&self.created_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MockRuleRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) endpoint_id: Option<String>,
    pub(crate) name: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) match_query_json: String,
    pub(crate) match_headers_json: String,
    pub(crate) response_status: i64,
    pub(crate) response_headers_json: String,
    pub(crate) response_body_template: String,
    pub(crate) delay_ms: i64,
    pub(crate) enabled: i64,
    pub(crate) priority: i64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl MockRuleRow {
    pub(crate) fn from_model(model: &MockRule) -> Self {
        MockRuleRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            endpoint_id: model.endpoint_id.map(|id| id.to_string()),
            name: model.name.clone(),
            method: model.method.as_str().to_string(),
            path: model.path.clone(),
            match_query_json: serde_json::to_string(&model.match_query)
                .unwrap_or_else(|_| "[]".into()),
            match_headers_json: serde_json::to_string(&model.match_headers)
                .unwrap_or_else(|_| "[]".into()),
            response_status: model.response_status as i64,
            response_headers_json: serde_json::to_string(&model.response_headers)
                .unwrap_or_else(|_| "{}".into()),
            response_body_template: model.response_body_template.clone(),
            delay_ms: model.delay_ms as i64,
            enabled: if model.enabled { 1 } else { 0 },
            priority: model.priority,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<MockRule> {
        Ok(MockRule {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            endpoint_id: match self.endpoint_id {
                Some(id) => Some(parse_uuid(&id)?),
                None => None,
            },
            name: self.name,
            method: self.method.parse()?,
            path: self.path,
            match_query: serde_json::from_str(&self.match_query_json).unwrap_or_default(),
            match_headers: serde_json::from_str(&self.match_headers_json).unwrap_or_default(),
            response_status: self.response_status as u16,
            response_headers: serde_json::from_str(&self.response_headers_json).unwrap_or_default(),
            response_body_template: self.response_body_template,
            delay_ms: self.delay_ms as u64,
            enabled: self.enabled != 0,
            priority: self.priority,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct ResponseExampleRow {
    pub(crate) id: String,
    pub(crate) endpoint_id: String,
    pub(crate) name: String,
    pub(crate) status: i64,
    pub(crate) headers_json: String,
    pub(crate) body: String,
    pub(crate) content_type: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl ResponseExampleRow {
    pub(crate) fn from_model(model: &ResponseExample) -> Self {
        ResponseExampleRow {
            id: model.id.to_string(),
            endpoint_id: model.endpoint_id.to_string(),
            name: model.name.clone(),
            status: model.status as i64,
            headers_json: serde_json::to_string(&model.headers).unwrap_or_else(|_| "{}".into()),
            body: model.body.clone(),
            content_type: model.content_type.clone(),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    pub(crate) fn into_model(self) -> Result<ResponseExample> {
        Ok(ResponseExample {
            id: parse_uuid(&self.id)?,
            endpoint_id: parse_uuid(&self.endpoint_id)?,
            name: self.name,
            status: self.status as u16,
            headers: serde_json::from_str(&self.headers_json).unwrap_or_default(),
            body: self.body,
            content_type: self.content_type,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct WsMessageRow {
    pub(crate) id: String,
    pub(crate) message_type: String,
    pub(crate) payload: String,
    pub(crate) created_at: String,
}

impl WsMessageRow {
    pub(crate) fn into_model(self) -> Result<WsMessageRecord> {
        Ok(WsMessageRecord {
            id: parse_uuid(&self.id)?,
            message_type: match self.message_type.as_str() {
                "text" => WsMessageType::Text,
                "binary" => WsMessageType::Binary,
                "ping" => WsMessageType::Ping,
                other => {
                    return Err(AppError::Validation(format!("无效的消息类型：{other}")));
                }
            },
            payload: self.payload,
            created_at: parse_time(&self.created_at)?,
        })
    }
}

/// 环境变量加密（AES-256-GCM，密钥见 fox-secret）。
pub(crate) fn encrypt_env_json(vars: &HashMap<String, String>) -> String {
    let json = serde_json::to_string(vars).unwrap_or_else(|_| "{}".into());
    match fox_secret::ensure_master_key().and_then(|k| fox_secret::encrypt(&k, &json)) {
        Ok(cipher) => cipher,
        Err(_) => json,
    }
}

/// 环境变量解密。
///
/// 旧版本明文数据原样返回；明确加密格式但解密失败（主密钥丢失 / 更换、
/// 密文损坏）返回 `AppError::Decryption`，由 UI 层弹窗提示，
/// 避免把 base64 密文当明文解析成空变量而静默丢失。
pub(crate) fn decrypt_env_json(json: &str) -> Result<HashMap<String, String>> {
    let plain = fox_secret::ensure_master_key()
        .and_then(|k| fox_secret::decrypt(&k, json))
        .map_err(|e| AppError::Decryption(e.to_string()))?;
    serde_json::from_str(&plain)
        .map_err(|_| AppError::Decryption("环境变量密文已损坏，无法解析".to_string()))
}

pub(crate) fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| AppError::Validation(format!("无效 ID：{s}（{e}）")))
}

pub(crate) fn parse_time(s: &str) -> Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError::Validation(format!("无效时间：{s}（{e}）")))
}
