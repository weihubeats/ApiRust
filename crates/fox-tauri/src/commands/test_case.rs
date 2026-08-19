//! 测试用例 Command：列表（按接口）/ 保存 / 更新（名称·分组·请求内容·运行状态）/ 删除。

use tauri::State;
use uuid::Uuid;

use fox_core::model::{HttpMethod, KeyValue, TestCase, TestCaseStatus};
use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 允许的用例分组。
const CATEGORIES: [&str; 5] = ["正向", "负向", "边界值", "安全性", "其他"];

/// 列出接口的全部测试用例。
#[tauri::command(rename_all = "camelCase")]
pub async fn list_test_cases(
    state: State<'_, AppState>,
    request_id: Uuid,
) -> CommandResult<Vec<TestCase>> {
    repo::list_test_cases(&state.db, request_id)
        .await
        .map_err(Into::into)
}

/// 保存测试用例（新建）。名称必填、分组须在允许集合内。
#[tauri::command(rename_all = "camelCase")]
pub async fn save_test_case(
    state: State<'_, AppState>,
    test_case: TestCase,
) -> CommandResult<TestCase> {
    if test_case.name.trim().is_empty() {
        return Err(CommandError::validation("用例名称不能为空"));
    }
    if !CATEGORIES.contains(&test_case.category.as_str()) {
        return Err(CommandError::validation(format!(
            "无效的用例分组：{}",
            test_case.category
        )));
    }
    repo::create_test_case(&state.db, &test_case).await?;
    Ok(test_case)
}

/// 更新用例名称与分组（编辑入口）。
#[tauri::command(rename_all = "camelCase")]
pub async fn update_test_case_meta(
    state: State<'_, AppState>,
    case_id: Uuid,
    name: String,
    category: String,
) -> CommandResult<()> {
    if name.trim().is_empty() {
        return Err(CommandError::validation("用例名称不能为空"));
    }
    if !CATEGORIES.contains(&category.as_str()) {
        return Err(CommandError::validation(format!(
            "无效的用例分组：{category}"
        )));
    }
    repo::update_test_case_meta(&state.db, case_id, &name, &category)
        .await
        .map_err(Into::into)
}

/// 更新用例运行状态（Success / Failed / Untested）。
#[tauri::command(rename_all = "camelCase")]
pub async fn update_test_case_status(
    state: State<'_, AppState>,
    case_id: Uuid,
    status: TestCaseStatus,
) -> CommandResult<()> {
    repo::update_test_case_status(&state.db, case_id, status)
        .await
        .map_err(Into::into)
}

/// 更新用例完整请求内容（方法 / 路径 / Params / Headers / Body，抽屉「保存修改」）。
#[tauri::command(rename_all = "camelCase")]
#[allow(clippy::too_many_arguments)]
pub async fn update_test_case_content(
    state: State<'_, AppState>,
    case_id: Uuid,
    method: HttpMethod,
    url_path: String,
    params: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body_type: String,
    body_content: String,
) -> CommandResult<()> {
    if url_path.trim().is_empty() {
        return Err(CommandError::validation("请求路径不能为空"));
    }
    repo::update_test_case_content(
        &state.db,
        case_id,
        method,
        &url_path,
        &params,
        &headers,
        &body_type,
        &body_content,
    )
    .await
    .map_err(Into::into)
}

/// 删除测试用例。
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_test_case(state: State<'_, AppState>, case_id: Uuid) -> CommandResult<()> {
    repo::delete_test_case(&state.db, case_id)
        .await
        .map_err(Into::into)
}
