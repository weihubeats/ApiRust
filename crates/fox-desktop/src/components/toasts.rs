//! 全局 Toast 容器。

use dioxus::prelude::*;

use crate::state::AppState;

#[component]
pub fn Toasts() -> Element {
    let state = use_context::<AppState>();
    let items = state.toasts.read().clone();

    rsx! {
        div { class: "rf-toast-wrap",
            for t in items {
                div { class: "rf-toast {t.kind.css_class()}", "{t.message}" }
            }
        }
    }
}
