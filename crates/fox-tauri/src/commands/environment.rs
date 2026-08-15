//! 环境 Command：列表 / 保存 / 激活切换。

use tauri::State;
use uuid::Uuid;

use fox_core::model::Environment;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出项目下的全部环境。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_environments(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> CommandResult<Vec<Environment>> {
    repo::list_environments(&state.db, project_id)
        .await
        .map_err(Into::into)
}

/// 保存环境（upsert）。名称必填。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_environment(
    state: State<'_, AppState>,
    environment: Environment,
) -> CommandResult<Environment> {
    if environment.name.trim().is_empty() {
        return Err(CommandError::validation("环境名称不能为空"));
    }
    repo::save_environment(&state.db, &environment).await?;
    Ok(environment)
}

/// 切换激活环境（`null` 表示不使用环境变量）。返回切换后的环境缓存。
#[tauri::command(rename_all = "camelCase")]
pub async fn set_active_environment(
    state: State<'_, AppState>,
    environment_id: Option<Uuid>,
) -> CommandResult<Option<Environment>> {
    state.set_active_environment(environment_id).await?;
    state.active_environment().await
}

/// 读取当前激活环境。
#[tauri::command(rename_all = "camelCase")]
pub async fn get_active_environment(
    state: State<'_, AppState>,
) -> CommandResult<Option<Environment>> {
    state.active_environment().await
}

/// 删除环境；若删除的是当前激活环境，则同时清空激活状态。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_environment(
    state: State<'_, AppState>,
    environment_id: Uuid,
) -> CommandResult<()> {
    repo::delete_environment(&state.db, environment_id).await?;
    let mut active = state.active.write().await;
    if active.environment_id == Some(environment_id) {
        active.environment_id = None;
        active.environment = None;
    }
    Ok(())
}
