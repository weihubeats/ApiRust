//! # fox-tauri：fox-core / fox-storage 的 Tauri 2 插件封装
//!
//! 将数据库访问与请求执行封装为 Tauri Command，供 Vue 3 前端通过
//! `@tauri-apps/api/core` 的 `invoke()` 调用。
//!
//! ## 接入方式（Tauri 应用）
//!
//! ```rust,ignore
//! fn main() {
//!     tauri::Builder::default()
//!         .plugin(fox_tauri::plugin::init())
//!         .run(tauri::generate_context!())
//!         .expect("error while running tauri application");
//! }
//! ```
//!
//! 插件初始化流程：
//! 1. 打开 / 创建 `{data_dir}/RustFox/rustfox.db` 并执行迁移（`fox_storage::db::init_db`）；
//! 2. `app.manage(AppState)` 托管全局状态（连接池 + `tokio::sync::RwLock` 激活上下文）；
//! 3. 注册全部 Command（见下方 [`commands`]）。
//!
//! ## 错误约定
//!
//! 所有 Command 返回 `Result<T, CommandError>`；失败时前端 `invoke()` reject
//! 一个 `{ code: string, message: string }` 对象（`code` 如 `VALIDATION`/`NOT_FOUND`）。
//!
//! ## TypeScript 类型同步（.d.ts 生成方案）
//!
//! 方案 A（推荐）：`tauri-specta` — 在插件里对 Command 声明做 `collect_commands!`，
//! 构建期导出 `bindings.ts`（命令签名 + 实体类型），前端类型与 Rust 严格一致：
//!
//! ```rust,ignore
//! #[cfg(feature = "specta")]
//! tauri_specta::Builder::<tauri::Wry>::new()
//!     .commands(tauri_specta::collect_commands![
//!         get_projects, save_project, delete_project, set_active_project,
//!         list_endpoints, get_endpoint, save_endpoint, delete_endpoint, duplicate_endpoint,
//!         list_environments, save_environment, set_active_environment,
//!         execute_request,
//!     ])
//!     .export(specta_typescript::Typescript::default(), "bindings.ts")
//!     .expect("failed to export specta bindings");
//! ```
//!
//! 注意：模型类型（`fox_core::model::*`）需要 `specta::Type` 派生，建议在
//! `fox-core` 增加可选 `specta` feature 后在模型上 `#[cfg_attr(feature = "specta", derive(specta::Type))]`。
//!
//! 方案 B（零依赖）：手工维护 `frontend/src/types/foxApi.d.ts`（本仓已提供一份镜像），
//! 并在 `useFoxApi.ts` 中统一入口，保证单点修改。

pub mod commands;
pub mod error;
pub mod state;

use tauri::Manager;

pub use error::{CommandError, CommandResult};
pub use state::AppState;

/// 插件命名空间。`init()` 注册状态与全部 Command。
pub mod plugin {
    use super::*;

    /// 注册 Fox 核心插件。
    pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
        tauri::plugin::Builder::new("fox")
            .setup(|app: &tauri::AppHandle<R>, _api: tauri::plugin::PluginApi<R, ()>| {
                // 初始化数据库（建目录 + 迁移）。阻塞主线程代价低（本地 SQLite）。
                let db = tauri::async_runtime::block_on(fox_storage::db::init_db(
                    &fox_storage::db::database_path(),
                ))
                .map_err(CommandError::from)?;
                app.manage(AppState::new(db));
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                commands::get_projects,
                commands::save_project,
                commands::delete_project,
                commands::set_active_project,
                commands::get_active_project,
                commands::list_endpoints,
                commands::get_endpoint,
                commands::save_endpoint,
                commands::delete_endpoint,
                commands::duplicate_endpoint,
                commands::list_environments,
                commands::save_environment,
                commands::set_active_environment,
                commands::get_active_environment,
                commands::execute_request,
            ])
            .build()
    }
}
