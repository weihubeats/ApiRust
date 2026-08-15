//! 请求执行 Command：变量渲染 → 参数校验 → 发送 HTTP 请求（成功后落历史）。

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use fox_core::model::{AuthSpec, BodySpec, HttpMethod, KeyValue, RequestHistory, RequestSpec};
use fox_core::VariableMap;
use fox_http::client::HttpResponseData;
use fox_storage::repository as repo;

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
    /// 历史归属项目 / 接口（可选；提供时成功后记入请求历史）。
    pub project_id: Option<Uuid>,
    pub endpoint_id: Option<Uuid>,
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
#[tauri::command(rename_all = "camelCase")]
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
    let parsed =
        url::Url::parse(&url).map_err(|e| CommandError::validation(format!("URL 无效：{e}")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(CommandError::validation(format!(
                "不支持的协议：{other}（仅支持 http/https）"
            )));
        }
    }

    // 3. 发送（超时取自 RequestSpec，默认 30s）。
    let resp: HttpResponseData =
        fox_http::client::send_request(args.method, &url, &spec, Some(spec.timeout_ms)).await?;

    // 4. 映射为可序列化响应。
    let body = resp.body_text();
    let content_type = resp.content_type();
    let response = ExecuteResponse {
        status: resp.status,
        headers: resp.headers,
        body,
        content_type,
        duration_ms: resp.duration_ms,
        size_bytes: resp.size_bytes,
        truncated: resp.truncated,
    };

    // 5. 写入请求历史（尽力而为：失败仅告警，不阻断发送）。
    if let Some(project_id) = args.project_id {
        let history = build_history(
            project_id,
            args.endpoint_id,
            args.method,
            &args.url,
            &response,
        );
        if let Err(e) = repo::save_request_history(&state.db, &history).await {
            eprintln!("[execute_request] 保存历史失败：{}", e.user_message());
        }
    }

    Ok(response)
}

/// 构建历史记录（字段与 Dioxus 版 `build_history` 对齐）。
fn build_history(
    project_id: Uuid,
    endpoint_id: Option<Uuid>,
    method: HttpMethod,
    url: &str,
    data: &ExecuteResponse,
) -> RequestHistory {
    let body_preview: String = data.body.chars().take(2000).collect();
    RequestHistory {
        id: Uuid::new_v4(),
        project_id,
        endpoint_id,
        method: method.to_string(),
        url: url.to_string(),
        status: Some(data.status),
        duration_ms: Some(data.duration_ms),
        request_summary_json: serde_json::json!({
            "method": method.to_string(),
            "url": url,
        })
        .to_string(),
        response_summary_json: serde_json::json!({
            "status": data.status,
            "duration_ms": data.duration_ms,
            "size_bytes": data.size_bytes,
            "truncated": data.truncated,
            "content_type": data.content_type,
            "body": body_preview,
        })
        .to_string(),
        created_at: chrono::Utc::now(),
    }
}

/// 渲染请求规格中的全部变量（key/value、认证、body）。
pub(crate) fn render_spec(spec: &RequestSpec, vars: &VariableMap) -> RequestSpec {
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
            AuthSpec::OAuth2 {
                client_id,
                client_secret,
                auth_url,
                token_url,
                scope,
                redirect_uri,
                token,
            } => AuthSpec::OAuth2 {
                client_id: fox_core::resolve_variables(client_id, vars),
                client_secret: fox_core::resolve_variables(client_secret, vars),
                auth_url: fox_core::resolve_variables(auth_url, vars),
                token_url: fox_core::resolve_variables(token_url, vars),
                scope: fox_core::resolve_variables(scope, vars),
                redirect_uri: fox_core::resolve_variables(redirect_uri, vars),
                token: token.clone(),
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
            BodySpec::GraphQL { spec } => BodySpec::GraphQL {
                spec: fox_core::model::GraphQLSpec {
                    query: fox_core::resolve_variables(&spec.query, vars),
                    variables: fox_core::resolve_variables(&spec.variables, vars),
                    operation_name: fox_core::resolve_variables(&spec.operation_name, vars),
                },
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
