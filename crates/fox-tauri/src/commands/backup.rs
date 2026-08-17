//! 备份/恢复 Command:项目全量 JSON 导出(下载)与导入(新项目 UUID 重映射)。
//! 对应 Dioxus 版 M10 备份功能。

use tauri::State;
use uuid::Uuid;

use fox_backup::{build_backup, restore_backup, BackupFile};
use fox_core::model::EndpointStatus;
use fox_storage::repository as repo;

use crate::error::CommandResult;
use crate::state::AppState;

/// 导出项目为备份 JSON 字符串（含项目 + 文件夹 + 接口 + 环境 + Mock 规则 + 响应示例 + 请求用例）。
#[tauri::command(rename_all = "camelCase")]
pub async fn backup_export(state: State<'_, AppState>, project_id: Uuid) -> CommandResult<String> {
    let project = repo::get_project(&state.db, project_id).await?;
    let folders = repo::list_folders(&state.db, project_id).await?;
    let endpoints = repo::list_endpoints(&state.db, project_id).await?;
    let environments = repo::list_environments(&state.db, project_id).await?;
    let mock_rules = repo::list_mock_rules(&state.db, project_id).await?;

    let mut response_examples = Vec::new();
    let mut request_examples = Vec::new();
    for ep in endpoints
        .iter()
        .filter(|e| e.status != EndpointStatus::Deprecated)
    {
        if let Ok(list) = repo::list_response_examples(&state.db, ep.id).await {
            response_examples.extend(list);
        }
        if let Ok(list) = repo::list_request_examples(&state.db, ep.id).await {
            request_examples.extend(list);
        }
    }

    let file = build_backup(
        &project,
        &folders,
        &endpoints,
        &environments,
        &mock_rules,
        &response_examples,
        &request_examples,
    );
    file.serialize().map_err(Into::into)
}

/// 从备份 JSON 恢复：校验格式 → 全量重映射 UUID → 落库为全新项目。
/// 返回 `{ id, name, counts }` 摘要。
#[tauri::command(rename_all = "camelCase")]
pub async fn backup_restore(
    state: State<'_, AppState>,
    text: String,
) -> CommandResult<serde_json::Value> {
    let file = BackupFile::parse(&text)?;
    let restored = restore_backup(&file);

    repo::save_project(&state.db, &restored.project).await?;
    for folder in &restored.folders {
        repo::save_folder(&state.db, folder).await?;
    }
    for endpoint in &restored.endpoints {
        repo::save_endpoint(&state.db, endpoint).await?;
    }
    for environment in &restored.environments {
        repo::save_environment(&state.db, environment).await?;
    }
    for rule in &restored.mock_rules {
        repo::save_mock_rule(&state.db, rule).await?;
    }
    for example in &restored.response_examples {
        repo::save_response_example(&state.db, example).await?;
    }
    for example in &restored.request_examples {
        repo::create_request_example(&state.db, example).await?;
    }

    Ok(serde_json::json!({
        "id": restored.project.id,
        "name": restored.project.name,
        "folders": restored.folders.len(),
        "endpoints": restored.endpoints.len(),
        "environments": restored.environments.len(),
        "mock_rules": restored.mock_rules.len(),
        "response_examples": restored.response_examples.len(),
        "request_examples": restored.request_examples.len(),
    }))
}
