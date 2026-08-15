//! 文件夹 Command：列表 / 保存（新建、重命名）/ 删除。

use tauri::State;
use uuid::Uuid;

use fox_core::model::Folder;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出项目下的全部文件夹（含父子层级，由前端组装树）。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_folders(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> CommandResult<Vec<Folder>> {
    repo::list_folders(&state.db, project_id)
        .await
        .map_err(Into::into)
}

/// 保存文件夹（upsert）：新建传新 id，重命名传已存在 id。名称必填。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_folder(state: State<'_, AppState>, folder: Folder) -> CommandResult<Folder> {
    if folder.name.trim().is_empty() {
        return Err(CommandError::validation("文件夹名称不能为空"));
    }
    repo::save_folder(&state.db, &folder).await?;
    Ok(folder)
}

/// 删除文件夹（级联删除子树文件夹与子树下全部接口）。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_folder(state: State<'_, AppState>, folder_id: Uuid) -> CommandResult<()> {
    repo::delete_folder(&state.db, folder_id)
        .await
        .map_err(Into::into)
}
