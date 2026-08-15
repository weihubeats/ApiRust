//! 响应示例 Command:列表(按接口)/ 保存(新建·覆盖)/ 删除。
//! 对应 Dioxus 版的「示例保存」:执行响应一键存档,树/标签页可直接回看。

use tauri::State;
use uuid::Uuid;

use fox_core::model::ResponseExample;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出接口的全部响应示例（按创建时间倒序）。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_examples(
    state: State<'_, AppState>,
    endpoint_id: Uuid,
) -> CommandResult<Vec<ResponseExample>> {
    repo::list_response_examples(&state.db, endpoint_id)
        .await
        .map_err(Into::into)
}

/// 保存响应示例（upsert）：新建传新 id，覆盖传已存在 id。名称必填。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_example(
    state: State<'_, AppState>,
    example: ResponseExample,
) -> CommandResult<ResponseExample> {
    if example.name.trim().is_empty() {
        return Err(CommandError::validation("示例名称不能为空"));
    }
    repo::save_response_example(&state.db, &example).await?;
    Ok(example)
}

/// 删除响应示例。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_example(state: State<'_, AppState>, example_id: Uuid) -> CommandResult<()> {
    repo::delete_response_example(&state.db, example_id)
        .await
        .map_err(Into::into)
}
