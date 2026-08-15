//! 导入导出 Command：OpenAPI 3.0 / Swagger 2.0 / Postman v2.1 导入（解析预览），
//! 项目接口导出为 OpenAPI 3.0 JSON。

use std::collections::HashMap;

use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use fox_core::model::EndpointStatus;
use fox_openapi::import::{import_any, ImportFormat};
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 导入结果（前端先预览，确认后才落库）。
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub format: ImportFormat,
    pub endpoints: Vec<fox_openapi::import::ImportedEndpoint>,
}

/// 解析文档文本：自动识别格式并提取接口（不落库）。
#[tauri::command(rename_all = "camelCase")]
pub async fn import_document(
    _state: State<'_, AppState>,
    text: String,
) -> CommandResult<ImportResult> {
    let (endpoints, format) = import_any(&text).map_err(CommandError::from)?;
    if endpoints.is_empty() {
        return Err(CommandError::validation("文档中没有可导入的接口"));
    }
    Ok(ImportResult { format, endpoints })
}

/// 导出项目接口为 OpenAPI 3.0 JSON 文本（含响应示例）。
#[tauri::command(rename_all = "camelCase")]
pub async fn export_openapi(state: State<'_, AppState>, project_id: Uuid) -> CommandResult<String> {
    let project = repo::get_project(&state.db, project_id).await?;
    let endpoints = repo::list_endpoints(&state.db, project_id).await?;

    let mut examples_by_endpoint: HashMap<Uuid, Vec<fox_core::model::ResponseExample>> =
        HashMap::new();
    for ep in endpoints
        .iter()
        .filter(|e| e.status != EndpointStatus::Deprecated)
    {
        if let Ok(list) = repo::list_response_examples(&state.db, ep.id).await {
            examples_by_endpoint.insert(ep.id, list);
        }
    }

    fox_openapi::export::export_project(&project.name, &endpoints, &examples_by_endpoint)
        .map_err(CommandError::from)
}
