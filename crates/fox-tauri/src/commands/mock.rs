//! Mock 服务 Command:启动(规则+接口示例生成定义)/ 停止 / 查询状态。
//! 端口从 4010 起自动探测;启动逻辑与 Dioxus 版对齐:启用规则优先,接口兜底。

use tauri::State;
use uuid::Uuid;

use fox_core::model::EndpointStatus;
use fox_mock::server::MockServer;
use fox_storage::repository as repo;
use std::collections::HashMap;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 启动 Mock 服务（重复启动返回 409 语义错误码 MOCK_RUNNING）。
#[tauri::command(rename_all = "camelCase")]
pub async fn mock_start(state: State<'_, AppState>) -> CommandResult<String> {
    {
        let running = state.mock.read().await;
        if running.is_some() {
            return Err(CommandError::with_code(
                "MOCK_RUNNING",
                "Mock 服务已在运行，请先停止",
            ));
        }
    }

    let project = state
        .active_project()
        .await?
        .ok_or_else(|| CommandError::validation("未选择项目，无法启动 Mock 服务"))?;
    let project_id: Uuid = project.id;

    // 1. 加载规则与接口示例，构建定义（规则 > 示例 > 默认，与 Dioxus 版一致）。
    let rules = repo::list_mock_rules(&state.db, project_id).await?;
    let endpoints = repo::list_endpoints(&state.db, project_id).await?;
    let mut examples_by_ep: HashMap<Uuid, Vec<fox_core::model::ResponseExample>> = HashMap::new();
    for ep in endpoints
        .iter()
        .filter(|e| e.status != EndpointStatus::Deprecated)
    {
        if let Ok(list) = repo::list_response_examples(&state.db, ep.id).await {
            examples_by_ep.insert(ep.id, list);
        }
    }

    let mut defs: Vec<fox_mock::server::MockDefinition> = Vec::new();
    for rule in rules.iter().filter(|r| r.enabled) {
        defs.push(fox_mock::server::MockDefinition::from_rule(rule));
    }
    for ep in endpoints
        .iter()
        .filter(|e| e.status != EndpointStatus::Deprecated)
    {
        let example = examples_by_ep.get(&ep.id).and_then(|l| l.first());
        defs.push(fox_mock::server::MockDefinition::from_endpoint(
            ep.method.as_str(),
            &ep.path,
            example,
        ));
    }

    // 2. 启动并托管句柄。
    let store = fox_mock::server::MockStore::new();
    store.set_definitions(defs);
    let server = fox_mock::server::start(store).await?;
    let address = server.address();
    *state.mock.write().await = Some(server);
    Ok(address)
}

/// 停止 Mock 服务（幂等）。
#[tauri::command(rename_all = "camelCase")]
pub async fn mock_stop(state: State<'_, AppState>) -> CommandResult<()> {
    let mut guard = state.mock.write().await;
    if let Some(server) = guard.take() {
        server.stop().await;
    }
    Ok(())
}

/// Mock 服务状态：未运行返回 `None`，运行中返回监听地址。
#[tauri::command(rename_all = "camelCase")]
pub async fn mock_status(state: State<'_, AppState>) -> CommandResult<Option<String>> {
    let running = state.mock.read().await;
    Ok(running.as_ref().map(MockServer::address))
}
