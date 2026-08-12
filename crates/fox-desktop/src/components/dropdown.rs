//! 自定义下拉组件（rf-dropdown），替代原生下拉选择器。
//!
//! 行为：
//! - 点击 trigger 切换展开；展开时渲染全屏 backdrop + 菜单；
//! - 点击 backdrop 或按 Esc 关闭；点击选项触发 on_select 并关闭；
//! - 未选择时 trigger 显示占位文案（颜色 var(--muted)）。

use dioxus::prelude::*;

use super::icons::CaretIcon;

#[component]
pub fn Dropdown(
    /// 选项列表：(value, label)。
    options: Vec<(String, String)>,
    /// 当前选中 value（空字符串表示未选择）。
    selected: String,
    /// 未选择时占位文案。
    #[props(default)]
    placeholder: String,
    /// 选中回调（传出 value）。
    on_select: EventHandler<String>,
    /// 附加类（宽度等）。
    #[props(default)]
    class: String,
    /// 底部区域（如"管理环境"按钮），可选。
    #[props(default)]
    footer: Option<Element>,
) -> Element {
    let open = use_signal(|| false);

    let has_selection = !selected.is_empty() && options.iter().any(|(ov, _)| ov == &selected);
    let display = options
        .iter()
        .find(|(v, _)| v == &selected)
        .map(|(_, l)| l.clone())
        .or_else(|| {
            if has_selection {
                Some(selected.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            if placeholder.is_empty() {
                "请选择".into()
            } else {
                placeholder.clone()
            }
        });

    let container_class = {
        let mut c = format!("rf-dropdown {}", if *open.read() { "open" } else { "" });
        if !has_selection {
            c.push_str(" has-placeholder");
        }
        if !class.is_empty() {
            c.push(' ');
            c.push_str(&class);
        }
        c
    };

    rsx! {
        div { class: "{container_class}",
            button {
                class: "rf-dropdown-trigger",
                onclick: move |_| {
                    let mut o = open;
                    let cur = *o.peek();
                    o.set(!cur);
                },
                onkeydown: move |e| {
                    if e.data().key().to_string() == "Escape" {
                        let mut o = open;
                        o.set(false);
                    }
                },
                span {
                    class: if has_selection { "rf-dropdown-value" } else { "rf-dropdown-value rf-placeholder" },
                    "{display}"
                }
                span { class: "rf-caret",
                    CaretIcon {}
                }
            }
            if *open.read() {
                div {
                    class: "rf-dropdown-backdrop",
                    onclick: move |_| {
                        let mut o = open;
                        o.set(false);
                    },
                }
                div { class: "rf-dropdown-menu",
                    for (value, label) in options.into_iter() {
                        div {
                            class: if selected == value {
                                "rf-dropdown-item selected"
                            } else {
                                "rf-dropdown-item"
                            },
                            onclick: move |_| {
                                let mut o = open;
                                o.set(false);
                                let v = value.clone();
                                on_select.call(v);
                            },
                            "{label}"
                        }
                    }
                    if let Some(f) = footer {
                        { f }
                    }
                }
            }
        }
    }
}
