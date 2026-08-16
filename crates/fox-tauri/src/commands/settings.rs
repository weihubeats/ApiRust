//! HTTP 设置 Command：全局代理（持久化 + 应用到 fox-http 共享客户端）。

use tauri::State;

use fox_storage::repository as repo;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// settings 表中的代理键；值为 JSON 字符串（`null` = 直连）。
const PROXY_KEY: &str = "http_proxy";

/// 读取全局代理地址（None = 直连）。
#[tauri::command(rename_all = "camelCase")]
pub async fn get_http_proxy(state: State<'_, AppState>) -> CommandResult<Option<String>> {
    let raw = repo::get_setting(&state.db, PROXY_KEY)
        .await
        .map_err(CommandError::from)?;
    match raw {
        None => Ok(None),
        Some(json) => serde_json::from_str::<Option<String>>(&json)
            .map_err(|e| CommandError::with_code("INTERNAL", format!("代理设置解析失败：{e}"))),
    }
}

/// 设置全局代理（`http://host:port` / `socks5://host:port`；None = 直连）。
///
/// 持久化到 settings 并立即应用到共享 HTTP 客户端；应用启动时
/// （[`crate::state`] 初始化后）通过 [`apply_saved_proxy`] 恢复。
#[tauri::command(rename_all = "camelCase")]
pub async fn set_http_proxy(
    state: State<'_, AppState>,
    proxy: Option<String>,
) -> CommandResult<()> {
    let trimmed = proxy
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    if let Some(p) = &trimmed {
        // 提前校验格式，避免坏地址在每次发请求时才报错
        fox_http::client::validate_proxy(p)
            .map_err(|e| CommandError::validation(e.user_message()))?;
    }
    fox_http::client::set_proxy(trimmed.as_deref())
        .map_err(|e| CommandError::validation(e.user_message()))?;
    let json = serde_json::to_string(&trimmed)
        .map_err(|e| CommandError::with_code("INTERNAL", format!("序列化失败：{e}")))?;
    repo::set_setting(&state.db, PROXY_KEY, &json)
        .await
        .map_err(CommandError::from)?;
    Ok(())
}

/// 启动时恢复持久化的代理设置（设置加载失败时静默保持直连）。
pub async fn apply_saved_proxy(db: &sqlx::SqlitePool) {
    let raw = match repo::get_setting(db, PROXY_KEY).await {
        Ok(Some(json)) => json,
        _ => return,
    };
    if let Ok(Some(proxy)) = serde_json::from_str::<Option<String>>(&raw) {
        if !proxy.is_empty() {
            let _ = fox_http::client::set_proxy(Some(proxy.as_str()));
        }
    }
}
