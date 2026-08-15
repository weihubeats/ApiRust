//! 项目 Command：增删改查 + 激活上下文切换。

use tauri::State;
use uuid::Uuid;

use fox_core::model::Project;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出全部项目。
#[tauri::command(rename_all = "camelCase")]
pub async fn get_projects(state: State<'_, AppState>) -> CommandResult<Vec<Project>> {
    repo::list_projects(&state.db).await.map_err(Into::into)
}

/// 创建或覆盖保存项目。
///
/// 参数校验示例：项目名称必填。校验失败返回 `{ code: "VALIDATION", message }`。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_project(state: State<'_, AppState>, project: Project) -> CommandResult<Project> {
    if project.name.trim().is_empty() {
        return Err(CommandError::validation("项目名称不能为空"));
    }
    repo::save_project(&state.db, &project).await?;
    Ok(project)
}

/// 删除项目（同时清理激活上下文缓存）。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_project(state: State<'_, AppState>, project_id: Uuid) -> CommandResult<()> {
    repo::delete_project(&state.db, project_id).await?;
    let mut active = state.active.write().await;
    if active.project_id == Some(project_id) {
        active.project_id = None;
        active.project = None;
        active.environment_id = None;
        active.environment = None;
    }
    Ok(())
}

/// 切换激活项目（`null` 表示清空）。返回切换后的项目缓存。
#[tauri::command(rename_all = "camelCase")]
pub async fn set_active_project(
    state: State<'_, AppState>,
    project_id: Option<Uuid>,
) -> CommandResult<Option<Project>> {
    state.set_active_project(project_id).await?;
    state.active_project().await
}

/// 读取当前激活项目。
#[tauri::command(rename_all = "camelCase")]
pub async fn get_active_project(state: State<'_, AppState>) -> CommandResult<Option<Project>> {
    state.active_project().await
}
