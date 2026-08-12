//! 顶部栏：三栏 Flex 布局。
//! 左：Logo（狐狸 SVG + 字标）+ 面包屑（项目名）；中：全局搜索（max-width 480px，⌘K 胶囊）；
//! 右：反馈 / 设置（ghost 图标按钮 + tooltip）。

use dioxus::prelude::*;

use crate::components::icons::{BugIcon, FoxFaceIcon, SearchIcon, SlidersIcon};
use crate::feedback;
use crate::state::{AppState, Page};

#[component]
pub fn TopBar() -> Element {
    let state = use_context::<AppState>();

    let mut search = state.search;
    let current_project_id = *state.current_project_id.read();
    let project_name: String = current_project_id
        .and_then(|id| {
            state
                .projects
                .read()
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
        })
        .unwrap_or_default();
    let st_feedback = state.clone();

    // 快捷键提示：macOS 显示 ⌘K，其余显示 Ctrl+K。
    let shortcut = if std::env::consts::OS == "macos" {
        "⌘K".to_string()
    } else {
        "Ctrl+K".to_string()
    };

    rsx! {
        header { class: "rf-topbar",
            div { class: "tb-left",
                div {
                    class: "rf-logo",
                    onclick: move |_| {
                        let mut p = state.current_page;
                        p.set(Page::Home);
                    },
                    FoxFaceIcon {}
                    span { "RustFox" }
                }
                div { class: "rf-topbar-sep" }
                div { class: "tb-breadcrumb",
                    span { "项目" }
                    if !project_name.is_empty() {
                        span { class: "tb-breadcrumb-sep", "/" }
                        span { class: "tb-breadcrumb-current", "{project_name}" }
                    }
                }
            }
            div { class: "tb-center",
                div { class: "tb-search",
                    SearchIcon {}
                    input {
                        id: "global-search",
                        class: "rf-input topbar-search-input",
                        placeholder: "搜索接口",
                        value: "{search}",
                        oninput: move |e| search.set(e.data().value()),
                    }
                    span { class: "ws-kbd tb-kbd", "{shortcut}" }
                }
            }
            div { class: "tb-right",
                button {
                    class: "rf-btn rf-btn-ghost tb-btn",
                    title: "生成反馈报告（提交到 GitHub Issue）",
                    onclick: move |_| {
                        let st_fb = st_feedback.clone();
                        spawn(async move {
                            match feedback::generate_report(&st_fb) {
                                Ok(path) => st_fb.toast_success(format!(
                                    "反馈报告已生成：{}（请将该文件内容提交到 GitHub Issue）",
                                    path.display()
                                )),
                                Err(e) => st_fb.toast_error(format!("生成反馈报告失败：{e}")),
                            }
                        });
                    },
                    BugIcon {}
                }
                button {
                    class: "rf-btn rf-btn-ghost tb-btn",
                    title: "设置",
                    onclick: move |_| {
                        let mut p = state.current_page;
                        p.set(Page::Settings);
                    },
                    SlidersIcon {}
                }
            }
        }
    }
}
