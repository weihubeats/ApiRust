//! 全局加载遮罩：全屏半透明黑底 + 纯 CSS 旋转 Spinner + 加载文案。
//! 由 AppState 的 is_loading / loading_text 驱动，异步操作前后调用
//! `state.set_loading("正在同步接口...")` / `state.clear_loading()`。

use dioxus::prelude::*;

use crate::state::AppState;

#[component]
pub fn LoadingOverlay() -> Element {
    let state = use_context::<AppState>();

    if !*state.is_loading.read() {
        return None;
    }
    let text = state.loading_text.read().clone();

    rsx! {
        div { class: "loading-overlay",
            div { class: "loading-box",
                div { class: "loading-spinner" }
                if !text.is_empty() {
                    div { class: "loading-text", "{text}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::components::loading_overlay::LoadingOverlay;
    use crate::services::Services;
    use crate::state::AppState;

    /// 根组件：在 VirtualDom 作用域内创建状态并挂载 LoadingOverlay。
    /// 提供 show / clear 两个触发按钮，在事件处理器内（合法 runtime 上下文）
    /// 调用 set_loading / clear_loading 驱动遮罩。
    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let state = AppState::new(Services::new(pool.clone()));
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        let st = state.clone();
        use_context_provider(move || state);
        let st2 = st.clone();
        rsx! {
            button {
                id: "show-loading",
                onclick: move |_| st.set_loading("正在同步接口..."),
            }
            button {
                id: "clear-loading",
                onclick: move |_| st2.clear_loading(),
            }
            LoadingOverlay {}
        }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
    }

    fn texts(muts: &Mutations) -> Vec<String> {
        let mut out = Vec::new();
        for m in &muts.edits {
            match m {
                Mutation::SetText { value, .. }
                | Mutation::CreateTextNode { value, .. }
                | Mutation::HydrateText { value, .. } => out.push(value.clone()),
                _ => {}
            }
        }
        out
    }

    #[test]
    fn overlay_shows_and_hides_with_loading_state() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(async { fox_storage::db::memory_pool().await.unwrap() });
        POOL.with(|s| *s.borrow_mut() = Some(pool));

        rt.block_on(async {
            dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");

            // 初始：is_loading = false，不渲染遮罩（无文案；仅 show/clear 两个驱动按钮监听）。
            // 静态 id 随 LoadTemplate 内建不可见，按注册顺序取：show 在前、clear 在后。
            let clicks1: Vec<ElementId> = m1
                .edits
                .iter()
                .filter_map(|m| match m {
                    Mutation::NewEventListener { name, id, .. } if name == "click" => Some(*id),
                    _ => None,
                })
                .collect();
            assert!(!*state.is_loading.read(), "初始 is_loading 应为 false");
            assert_eq!(clicks1.len(), 2, "应有 show/clear 两个驱动按钮");
            let show_btn = clicks1[0];
            let clear_btn = clicks1[1];

            // set_loading：遮罩挂载 + 动态文案出现（静态类名/Spinner 随 LoadTemplate 内建）。
            dom.handle_event("click", mouse(), show_btn, true);
            let m2 = dom.render_immediate_to_vec();
            assert!(
                texts(&m2).iter().any(|t| t.contains("正在同步接口")),
                "应渲染加载文案，实际 edits：{:?}",
                m2.edits.iter().map(|m| format!("{m:?}")).collect::<Vec<_>>()
            );
            assert!(*state.is_loading.read(), "set_loading 后 is_loading 应为 true");
            assert_eq!(
                state.loading_text.read().clone(),
                "正在同步接口...",
                "loading_text 应被设置"
            );

            // clear_loading：遮罩被替换卸载。
            dom.handle_event("click", mouse(), clear_btn, true);
            let m3 = dom.render_immediate_to_vec();
            let removed = m3
                .edits
                .iter()
                .filter(|m| matches!(m, Mutation::ReplaceWith { .. }))
                .count();
            assert!(removed > 0, "clear_loading 后应卸载遮罩");
            assert!(!*state.is_loading.read(), "clear_loading 后 is_loading 应为 false");
        });
    }

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(SerializedMouseData::default())))
    }
}