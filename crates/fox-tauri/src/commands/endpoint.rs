//! 接口 Command：列表 / 详情 / 保存（含参数校验）/ 删除 / 复制。

use tauri::State;
use uuid::Uuid;

use fox_core::model::Endpoint;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出项目下的全部接口。
#[tauri::command]
pub async fn list_endpoints(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> CommandResult<Vec<Endpoint>> {
    repo::list_endpoints(&state.db, project_id)
        .await
        .map_err(Into::into)
}

/// 接口详情。
#[tauri::command]
pub async fn get_endpoint(state: State<'_, AppState>, endpoint_id: Uuid) -> CommandResult<Endpoint> {
    repo::get_endpoint(&state.db, endpoint_id)
        .await
        .map_err(Into::into)
}

/// 保存接口（upsert）。
///
/// 参数校验：名称必填、路径必填且以 `/` 开头。服务端强校验，前端仅做体验层校验。
#[tauri::command]
pub async fn save_endpoint(state: State<'_, AppState>, endpoint: Endpoint) -> CommandResult<Endpoint> {
    if endpoint.name.trim().is_empty() {
        return Err(CommandError::validation("接口名称不能为空"));
    }
    if endpoint.path.trim().is_empty() {
        return Err(CommandError::validation("接口路径不能为空"));
    }
    if !endpoint.path.trim().starts_with('/') {
        return Err(CommandError::validation("接口路径必须以 / 开头"));
    }
    repo::save_endpoint(&state.db, &endpoint).await?;
    Ok(endpoint)
}

/// 删除接口。
#[tauri::command]
pub async fn delete_endpoint(
    state: State<'_, AppState>,
    endpoint_id: Uuid,
) -> CommandResult<()> {
    repo::delete_endpoint(&state.db, endpoint_id)
        .await
        .map_err(Into::into)
}

/// 复制接口（返回新接口）。
#[tauri::command]
pub async fn duplicate_endpoint(
    state: State<'_, AppState>,
    endpoint_id: Uuid,
) -> CommandResult<Endpoint> {
    repo::duplicate_endpoint(&state.db, endpoint_id)
        .await
        .map_err(Into::into)
}
