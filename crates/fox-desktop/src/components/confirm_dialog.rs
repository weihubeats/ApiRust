//! 通用二次确认弹窗（删除等危险操作）。

use dioxus::prelude::*;

/// 确认框内容。
#[derive(Clone, PartialEq)]
pub struct ConfirmInfo {
    pub title: String,
    pub message: String,
    /// 确认按钮文案。
    pub confirm_text: String,
}

impl ConfirmInfo {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_text: "删除".into(),
        }
    }
}

/// 二次确认弹窗：`info` 为 Some 时显示「取消 / 确认」。
#[component]
pub fn ConfirmDialog(
    info: Option<ConfirmInfo>,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    #[allow(clippy::question_mark)]
    let Some(info) = info
    else {
        return rsx! {};
    };
    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |e| {
                e.stop_propagation();
                on_cancel.call(());
            },
            div {
                class: "modal",
                onclick: |e| e.stop_propagation(),
                h3 { "{info.title}" }
                p { class: "confirm-message", "{info.message}" }
                div { class: "rf-modal-actions",
                    button { class: "rf-btn", onclick: move |_| on_cancel.call(()), "取消" }
                    button { class: "rf-btn rf-btn-danger", onclick: move |_| on_confirm.call(()), "{info.confirm_text}" }
                }
            }
        }
    }
}
