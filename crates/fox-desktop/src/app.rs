//! 应用根组件。

use dioxus::prelude::*;
use fox_storage::repository as repo;
use uuid::Uuid;

use crate::components::{
    loading_overlay::LoadingOverlay, project_tree::SideBar, toasts::Toasts, topbar::TopBar,
};
use crate::pages::{home::HomePage, settings::SettingsPage, workspace::WorkspacePage};
use crate::services::Services;
use crate::shortcuts::KeyboardShortcuts;
use crate::state::{AppState, Page};
use crate::updater::UpdateModal;

static DB_POOL: std::sync::OnceLock<sqlx::SqlitePool> = std::sync::OnceLock::new();

pub fn provide_pool(pool: sqlx::SqlitePool) {
    let _ = DB_POOL.set(pool);
}

/// 订阅窗口事件：收到 `CloseRequested` 时把窗口几何与最大化状态
/// 写入 settings 表（`window_state` 键），下次启动恢复。
///
/// 回调闭包参数类型（`Event<UserWindowEvent>`）包含不公开的内部类型，
/// 利用闭包类型推断，无需显式标注。
fn subscribe_window_state() {
    let Some(pool) = DB_POOL.get().cloned() else {
        return;
    };
    let desktop = dioxus::desktop::window();
    let desktop_for_handler = desktop.clone();
    let _ = desktop.create_wry_event_handler(move |event, _| {
        if let dioxus::desktop::tao::event::Event::WindowEvent { event: we, .. } = event {
            if matches!(we, dioxus::desktop::WindowEvent::CloseRequested) {
                let state =
                    crate::window_state::WindowState::from_window(&desktop_for_handler.window);
                crate::window_state::WindowState::save(pool.clone(), state);
            }
        }
    });
}

/// 应用根组件。
#[allow(non_snake_case)]
pub fn App() -> Element {
    let pool = DB_POOL.get().expect("数据库尚未初始化").clone();
    let state = use_context_provider(|| AppState::new(Services::new(pool.clone())));

    // Dioxus 0.5 的 use_effect 没有依赖数组，每次组件 re-render 都会重跑。
    // 用载入标志保证首次挂载只执行一次，避免页面切换 / 项目选择等每次渲染
    // 都对 SQLite 发起查询风暴，并只注册一次窗口事件监听。
    let started = use_hook(|| std::cell::Cell::new(false));
    let state_for_effect = state.clone();
    use_effect(move || {
        if !started.get() {
            started.set(true);
            state_for_effect.refresh_projects();
            // 启动时恢复上次选中的环境
            {
                let state_env = state_for_effect.clone();
                let pool_restore = pool.clone();
                spawn(async move {
                    if let Ok(Some(raw)) =
                        repo::get_setting(&pool_restore, "current_environment_id").await
                    {
                        if let Ok(id) = raw.parse::<Uuid>() {
                            state_env.select_environment(Some(id));
                        }
                    }
                });
            }
            subscribe_window_state();
            // 启动时静默检查更新（仅在 release 构建生效，测试/开发不触发网络请求）。
            if !cfg!(debug_assertions) {
                state_for_effect.check_for_update(false);
            }
        }
    });

    let page = *state.current_page.read();
    let has_project = state.current_project_id.read().is_some();

    rsx! {
        div { class: "app",
            style { "{crate::styles::DESIGN_SYSTEM_CSS}" }
            TopBar {}
            div { class: "body",
                if has_project {
                    SideBar {}
                }
                main {
                    class: "main",
                    match page {
                        Page::Home => rsx! { HomePage {} },
                        Page::Workspace => rsx! { WorkspacePage {} },
                        Page::Settings => rsx! { SettingsPage {} },
                    }
                }
            }
            Toasts {}
            UpdateModal {}
            LoadingOverlay {}
            KeyboardShortcuts {}
        }
    }
}
