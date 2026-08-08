//! 应用根组件。

use dioxus::prelude::*;

use crate::components::{project_tree::SideBar, toasts::Toasts, topbar::TopBar};
use crate::pages::{home::HomePage, settings::SettingsPage, workspace::WorkspacePage};
use crate::services::Services;
use crate::state::{AppState, Page};

static DB_POOL: std::sync::OnceLock<sqlx::SqlitePool> = std::sync::OnceLock::new();

pub fn provide_pool(pool: sqlx::SqlitePool) {
    let _ = DB_POOL.set(pool);
}

/// 应用根组件。
#[allow(non_snake_case)]
pub fn App() -> Element {
    let pool = DB_POOL.get().expect("数据库尚未初始化").clone();
    let state = use_context_provider(|| AppState::new(Services::new(pool)));

    let state_for_effect = state.clone();
    use_effect(move || {
        state_for_effect.refresh_projects();
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
        }
    }
}
