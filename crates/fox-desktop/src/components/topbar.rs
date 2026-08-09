//! 顶部栏：logo | 分隔线 | 项目下拉 | 环境下拉 | spacer | 搜索框 | 反馈 | 设置。

use dioxus::prelude::*;

use crate::components::dropdown::Dropdown;
use crate::components::icons::{SearchIcon, SlidersIcon};
use crate::feedback;
use crate::state::{AppState, Page};

#[component]
pub fn TopBar() -> Element {
    let state = use_context::<AppState>();

    let mut search = state.search;
    let projects = state.projects;
    let current_project_id = state.current_project_id;
    let environments = state.environments;
    let current_environment_id = state.current_environment_id;

    let options: Vec<(String, String)> = projects
        .read()
        .iter()
        .map(|p| (p.id.to_string(), p.name.clone()))
        .collect();
    let selected: String = current_project_id
        .read()
        .map(|id| id.to_string())
        .unwrap_or_default();
    let st_dropdown = state.clone();

    let env_options: Vec<(String, String)> = environments
        .read()
        .iter()
        .map(|e| (e.id.to_string(), e.name.clone()))
        .collect();
    let env_selected: String = current_environment_id
        .read()
        .map(|id| id.to_string())
        .unwrap_or_default();
    let st_env = state.clone();
    let st_feedback = state.clone();

    rsx! {
        header { class: "rf-topbar",
            div { class: "rf-logo", "RustFox" }
            div { class: "rf-topbar-sep" }
            Dropdown {
                options,
                selected,
                placeholder: "未选择项目",
                on_select: move |v: String| {
                    if let Ok(id) = uuid::Uuid::parse_str(&v) {
                        st_dropdown.select_project(id);
                    }
                },
            }
            div { class: "rf-topbar-sep" }
            Dropdown {
                class: "rf-dd-env",
                options: env_options,
                selected: env_selected,
                placeholder: "未选环境",
                on_select: move |v: String| {
                    let id = uuid::Uuid::parse_str(&v).ok();
                    st_env.select_environment(id);
                },
            }
            div { class: "rf-topbar-spacer" }
            div { class: "rf-search",
                SearchIcon {}
                input {
                    class: "rf-input",
                    placeholder: "搜索接口",
                    value: "{search}",
                    oninput: move |e| search.set(e.data().value()),
                }
            }
            button {
                class: "rf-btn rf-btn-ghost",
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
                "反馈"
            }
            button {
                class: "rf-btn rf-btn-ghost",
                onclick: move |_| {
                    let mut p = state.current_page;
                    p.set(Page::Settings);
                },
                SlidersIcon {}
                "设置"
            }
        }
    }
}
