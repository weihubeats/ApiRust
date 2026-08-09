//! 请求执行 Command：变量渲染 → 参数校验 → 发送 HTTP 请求。

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use fox_core::model::{AuthSpec, BodySpec, HttpMethod, KeyValue, RequestSpec};
use fox_core::VariableMap;
use fox_http::client::HttpResponseData;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 执行请求的入参（前端结构体，与 `RequestSpec` 一致可 JSON 互传）。
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteRequestArgs {
    /// 请求 URL 模板，支持 `{{variable}}`。
    pub url: String,
    pub method: HttpMethod,
    pub spec: RequestSpec,
    /// 本次请求使用的环境（缺省使用当前激活环境）。
    pub environment_id: Option<Uuid>,
}

/// 执行结果（`HttpResponseData` 含非序列化 `Bytes`，此处转成 JSON 安全结构）。
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub content_type: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub truncated: bool,
}

/// 执行 HTTP 请求：加载变量 → 渲染 URL/Headers/Body → 参数校验 → 发送。
#[tauri::command]
pub async fn execute_request(
    state: State<'_, AppState>,
    args: ExecuteRequestArgs,
) -> CommandResult<ExecuteResponse> {
    // 1. 加载变量（环境 > 项目），渲染 URL 与请求规格。
    let vars = state.variables_for(args.environment_id).await?;
    let url = fox_core::resolve_variables(&args.url, &vars);
    let spec = render_spec(&args.spec, &vars);

    // 2. 参数校验：URL 必填、必须是 http/https。
    if url.trim().is_empty() {
        return Err(CommandError::validation("URL 不能为空"));
    }
    let parsed = url::Url::parse(&url).map_err(|e| CommandError::validation(format!("URL 无效：{e}")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(CommandError::validation(format!(
                "不支持的协议：{other}（仅支持 http/https）"
            )));
        }
    }

    // 3. 发送（超时取自 RequestSpec，默认 30s）。
    let resp: HttpResponseData = fox_http::client::send_request(
        args.method,
        &url,
        &spec,
        Some(spec.timeout_ms),
    )
    .await?;

    // 4. 映射为可序列化响应。
    let body = resp.body_text();
    let content_type = resp.content_type();
    Ok(ExecuteResponse {
        status: resp.status,
        headers: resp.headers,
        body,
        content_type,
        duration_ms: resp.duration_ms,
        size_bytes: resp.size_bytes,
        truncated: resp.truncated,
    })
}

/// 渲染请求规格中的全部变量（key/value、认证、body）。
fn render_spec(spec: &RequestSpec, vars: &VariableMap) -> RequestSpec {
    RequestSpec {
        params: render_kv(&spec.params, vars),
        headers: render_kv(&spec.headers, vars),
        path_variables: render_kv(&spec.path_variables, vars),
        auth: match &spec.auth {
            AuthSpec::None => AuthSpec::None,
            AuthSpec::Bearer { token } => AuthSpec::Bearer {
                token: fox_core::resolve_variables(token, vars),
            },
            AuthSpec::Basic { username, password } => AuthSpec::Basic {
                username: fox_core::resolve_variables(username, vars),
                password: fox_core::resolve_variables(password, vars),
            },
            AuthSpec::ApiKey {
                key,
                value,
                location,
            } => AuthSpec::ApiKey {
                key: fox_core::resolve_variables(key, vars),
                value: fox_core::resolve_variables(value, vars),
                location: *location,
            },
        },
        body: match &spec.body {
            BodySpec::None => BodySpec::None,
            BodySpec::Json { raw } => BodySpec::Json {
                raw: fox_core::resolve_variables(raw, vars),
            },
            BodySpec::Text { raw } => BodySpec::Text {
                raw: fox_core::resolve_variables(raw, vars),
            },
            BodySpec::UrlEncoded { fields } => BodySpec::UrlEncoded {
                fields: render_kv(fields, vars),
            },
            BodySpec::Multipart { fields } => BodySpec::Multipart {
                fields: fields
                    .iter()
                    .map(|f| fox_core::model::MultipartField {
                        key: fox_core::resolve_variables(&f.key, vars),
                        value_type: f.value_type,
                        value: fox_core::resolve_variables(&f.value, vars),
                        enabled: f.enabled,
                    })
                    .collect(),
            },
        },
        timeout_ms: spec.timeout_ms,
        follow_redirects: spec.follow_redirects,
        tests: spec.tests.clone(),
    }
}

/// 渲染键值对列表（Query / Header / Path 变量）。
fn render_kv(items: &[KeyValue], vars: &VariableMap) -> Vec<KeyValue> {
    items
        .iter()
        .map(|kv| KeyValue {
            key: fox_core::resolve_variables(&kv.key, vars),
            value: fox_core::resolve_variables(&kv.value, vars),
            enabled: kv.enabled,
            description: kv.description.clone(),
        })
        .collect()
}
