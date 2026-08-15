//! 测试 Command：单接口断言测试（`test_endpoint`）+ 并发压测（`load_test`）。

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use fox_core::model::{Endpoint, HttpMethod, RequestSpec};

use crate::commands::request::render_spec;
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// `test_endpoint` 入参。
#[derive(Debug, Clone, Deserialize)]
pub struct TestEndpointArgs {
    pub endpoint: Endpoint,
    pub url: String,
    pub environment_id: Option<Uuid>,
}

/// 运行接口配置的断言测试（tests 未配置时也走通，返回空结果集）。
#[tauri::command(rename_all = "camelCase")]
pub async fn test_endpoint(
    state: State<'_, AppState>,
    args: TestEndpointArgs,
) -> CommandResult<fox_test::runner::EndpointResult> {
    let vars = state.variables_for(args.environment_id).await?;
    let url = fox_core::resolve_variables(&args.url, &vars);
    let spec = render_spec(&args.endpoint.request, &vars);
    let mut runtime = std::collections::HashMap::new();
    let (result, _) = fox_test::runner::run_endpoint(
        &args.endpoint,
        &url,
        &spec,
        &mut runtime,
        Some(spec.timeout_ms),
    )
    .await;
    Ok(result)
}

/// 压测配置。
#[derive(Debug, Clone, Deserialize)]
pub struct LoadTestArgs {
    pub url: String,
    pub method: HttpMethod,
    pub spec: RequestSpec,
    pub environment_id: Option<Uuid>,
    /// 并发数（默认 20）。
    pub concurrency: Option<usize>,
    /// 总请求数（默认 200）。
    pub total: Option<usize>,
}

/// 并发压测：发送 `total` 次请求，最多 `concurrency` 个同时进行。
/// 进度经事件 `fox:load-progress` 实时推送（done/total/ok/failed）。
#[tauri::command(rename_all = "camelCase")]
pub async fn load_test(
    app: AppHandle,
    state: State<'_, AppState>,
    args: LoadTestArgs,
) -> CommandResult<fox_test::load::LoadResult> {
    if args.url.trim().is_empty() {
        return Err(CommandError::validation("URL 不能为空"));
    }
    let concurrency = args.concurrency.unwrap_or(20).clamp(1, 500);
    let total = args.total.unwrap_or(200).clamp(1, 100_000);
    let vars = state.variables_for(args.environment_id).await?;
    let url = fox_core::resolve_variables(&args.url, &vars);
    let spec = render_spec(&args.spec, &vars);
    let cfg = fox_test::load::LoadConfig { concurrency, total };
    let progress = move |p: fox_test::load::LoadProgress| {
        let _ = app.emit("fox:load-progress", &p);
    };
    Ok(fox_test::load::run_load(args.method, &url, &spec, &cfg, Some(&progress)).await)
}
