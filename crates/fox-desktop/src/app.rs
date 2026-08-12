//! 应用根组件。

use dioxus::events::eval;
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

/// 「跟随系统」主题：matchMedia 监听系统深浅色并把解析结果写入 <html> 的 data-theme。
/// 重复执行（主题/渲染变化重跑）会先移除旧监听再注册，保证只挂一个。
const THEME_AUTO_JS: &str = r#"(function(){
  if (window.__rfThemeMq) { window.__rfThemeMq.removeEventListener("change", window.__rfThemeFn); }
  var mq = window.matchMedia("(prefers-color-scheme: light)");
  var fn = function() {
    document.documentElement.setAttribute("data-theme", mq.matches ? "light" : "dark");
  };
  mq.addEventListener("change", fn);
  window.__rfThemeMq = mq;
  window.__rfThemeFn = fn;
  fn();
})();"#;

pub fn provide_pool(pool: sqlx::SqlitePool) {
    let _ = DB_POOL.set(pool);
}

/// 测试用：读取已注入的全局连接池。
#[cfg(test)]
pub fn debug_pool() -> Option<sqlx::SqlitePool> {
    DB_POOL.get().cloned()
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
            // 启动时恢复上次选定的主题（settings 表持久化）。
            {
                let state_theme = state_for_effect.clone();
                let pool_theme = pool.clone();
                spawn(async move {
                    if let Ok(Some(raw)) = repo::get_setting(&pool_theme, "theme").await {
                        state_theme.set_theme(raw);
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
    let raw_theme = state.theme.read().clone();
    // 显式主题（浅色/深色）声明在 .app 上补充首帧色（JS 落地前的兜底）；
    // 根节点 <html> 的 data-theme 由下面的 effect 用 JS 写入（跟随系统走 matchMedia）。
    let theme_attr = if raw_theme == crate::state::theme::AUTO {
        String::new()
    } else {
        raw_theme
    };

    // 主题落地 <html>：显式主题直接写入；跟随系统用 matchMedia 监听系统切换并实时写入。
    // dioxus 0.5 的 use_effect 每次渲染都重跑，用上次已应用值去重。
    let theme_applied = use_hook(|| std::cell::RefCell::new(String::new()));
    let state_theme_eff = state.clone();
    use_effect(move || {
        let mode = state_theme_eff.theme.read().clone();
        if *theme_applied.borrow() == mode {
            return;
        }
        *theme_applied.borrow_mut() = mode.clone();
        let script = if mode == crate::state::theme::AUTO {
            THEME_AUTO_JS.to_string()
        } else {
            format!(r#"document.documentElement.setAttribute("data-theme","{mode}");"#)
        };
        let _ = eval(&script);
    });

    rsx! {
        div {
            class: "app",
            "data-theme": "{theme_attr}",
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
