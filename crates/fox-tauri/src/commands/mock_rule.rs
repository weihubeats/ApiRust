//! Mock 规则 Command：列表 / 保存（upsert）/ 删除。
//! 规则在 mock_start 时按 priority 降序匹配，可覆盖接口默认示例。

use tauri::State;
use uuid::Uuid;

use fox_core::model::MockRule;
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 列出项目的全部 Mock 规则（priority 降序）。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_mock_rules(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> CommandResult<Vec<MockRule>> {
    repo::list_mock_rules(&state.db, project_id)
        .await
        .map_err(CommandError::from)
}

/// 保存 Mock 规则（upsert）：新规则传新 id，修改传已存在 id。名称必填。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_mock_rule(state: State<'_, AppState>, rule: MockRule) -> CommandResult<MockRule> {
    if rule.name.trim().is_empty() {
        return Err(CommandError::validation("规则名称不能为空"));
    }
    if rule.path.trim().is_empty() {
        return Err(CommandError::validation("匹配路径不能为空"));
    }
    let exists = repo::list_mock_rules(&state.db, rule.project_id)
        .await?
        .iter()
        .any(|r| r.id == rule.id);
    if exists {
        repo::update_mock_rule(&state.db, &rule).await?;
    } else {
        repo::create_mock_rule(&state.db, rule.project_id, &rule).await?;
    }
    Ok(rule)
}

/// 删除 Mock 规则。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_mock_rule(state: State<'_, AppState>, rule_id: Uuid) -> CommandResult<()> {
    repo::delete_mock_rule(&state.db, rule_id)
        .await
        .map_err(CommandError::from)
}
