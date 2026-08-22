//! 代码生成 Command:把接口请求渲染为 curl / python / js / go / java / php / rust 片段。

use fox_codegen::{GenRequest, Lang};
use fox_core::model::{AuthSpec, BodySpec, HttpMethod, KeyValue};
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// 渲染请求为指定语言代码。
#[tauri::command(rename_all = "camelCase")]
pub async fn codegen_render(
    _state: State<'_, AppState>,
    lang: String,
    method: HttpMethod,
    url: String,
    headers: Vec<KeyValue>,
    body: BodySpec,
    auth: AuthSpec,
) -> CommandResult<String> {
    let lang = Lang::from_str_cn(&lang).ok_or_else(|| CommandError::validation("未知代码语言"))?;
    let req = GenRequest {
        method: &method,
        url: &url,
        headers: &headers,
        body: &body,
        auth: &auth,
    };
    Ok(fox_codegen::render(lang, &req))
}
