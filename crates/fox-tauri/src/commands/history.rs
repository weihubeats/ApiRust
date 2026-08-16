//! 请求历史 Command：查询最近请求记录（execute_request 成功时自动落库）。

use tauri::State;
use uuid::Uuid;

use fox_core::model::RequestHistory;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 查询项目最近的请求历史（按时间倒序，默认 50 条）。
/// `endpoint_id` 提供时仅返回该接口的记录（前端「仅当前接口」过滤）。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_request_histories(
    state: State<'_, AppState>,
    project_id: Uuid,
    endpoint_id: Option<Uuid>,
    limit: Option<i64>,
) -> CommandResult<Vec<RequestHistory>> {
    let limit = limit.filter(|l| *l > 0).unwrap_or(50);
    repo::list_request_histories(&state.db, project_id, endpoint_id, limit)
        .await
        .map_err(CommandError::from)
}
