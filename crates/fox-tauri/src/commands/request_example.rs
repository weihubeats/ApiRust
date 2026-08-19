//! 请求用例 Command：列表（按接口）/ 保存（新建快照）/ 删除。
//!
//! 与响应示例（example.rs）对应：把当前编辑器的完整请求（RequestSpec）
//! 存档为用例，可一键回填编辑器复用。

use tauri::State;
use uuid::Uuid;

use fox_core::model::RequestExample;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出接口的全部请求用例（最新在前）。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_request_examples(
    state: State<'_, AppState>,
    endpoint_id: Uuid,
) -> CommandResult<Vec<RequestExample>> {
    repo::list_request_examples(&state.db, endpoint_id)
        .await
        .map_err(Into::into)
}

/// 保存请求用例（新建快照）。名称必填。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_request_example(
    state: State<'_, AppState>,
    example: RequestExample,
) -> CommandResult<RequestExample> {
    if example.name.trim().is_empty() {
        return Err(CommandError::validation("用例名称不能为空"));
    }
    repo::create_request_example(&state.db, &example).await?;
    Ok(example)
}

/// 删除请求用例。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_request_example(
    state: State<'_, AppState>,
    example_id: Uuid,
) -> CommandResult<()> {
    repo::delete_request_example(&state.db, example_id)
        .await
        .map_err(Into::into)
}
