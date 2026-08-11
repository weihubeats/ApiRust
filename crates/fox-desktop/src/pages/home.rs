//! 首页：空状态、项目卡片网格、项目创建表单、快捷键提示。

use std::collections::HashMap;

use dioxus::prelude::*;
use fox_core::model::Project;
use fox_storage::repository as repo;
use uuid::Uuid;

use crate::components::confirm_dialog::{ConfirmDialog, ConfirmInfo};
use crate::components::icons::{FolderIcon, PlusIcon};
use crate::state::AppState;

fn format_time(t: &chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 加载每个项目的接口数（仅 UI 展示用）。
fn load_endpoint_counts(state: AppState, mut counts: Signal<HashMap<Uuid, usize>>) {
    let db = state.services.db.clone();
    let project_ids: Vec<Uuid> = state.projects.read().iter().map(|p| p.id).collect();
    spawn(async move {
        let mut map = HashMap::new();
        for id in project_ids {
            if let Ok(list) = repo::list_endpoints(&db, id).await {
                map.insert(id, list.len());
            }
        }
        counts.set(map);
    });
}

#[component]
pub fn HomePage() -> Element {
    let state = use_context::<AppState>();
    let projects = state.projects.read().clone();

    let mut new_name = use_signal(String::new);
    let mut new_desc = use_signal(String::new);
    let counts: Signal<HashMap<Uuid, usize>> = use_signal(HashMap::new);
    // 删除项目二次确认弹窗（保存待删除的项目 id，确认后才真正执行）。
    let confirm_del: Signal<Option<(ConfirmInfo, Uuid)>> = use_signal(|| None);

    {
        let st = state.clone();
        use_effect(move || {
            load_endpoint_counts(st.clone(), counts);
        });
    }

    // 每张卡片持有自己的克隆，避免闭包竞争。
    let handles: Vec<(Project, AppState, Uuid, String)> = projects
        .iter()
        .map(|p| (p.clone(), state.clone(), p.id, p.name.clone()))
        .collect();

    let count_map = counts.read().clone();
    let st_del = state.clone();

    rsx! {
        div { class: "rf-home",
            div { class: "rf-home-card rf-card",
                if projects.is_empty() {
                    div { class: "rf-empty",
                        div { class: "rf-empty-icon",
                            FolderIcon {}
                        }
                        div { class: "rf-empty-title", "还没有项目" }
                        div { class: "rf-empty-desc", "创建你的第一个项目，开始管理 API" }
                    }
                } else {
                    div { class: "rf-card-title", "项目列表" }
                    div { class: "rf-project-grid",
                        for (p, st_open, pid, p_name) in handles {
                            div {
                                key: "{p.id}",
                                class: "rf-project-card",
                                onclick: move |_| st_open.select_project(pid),
                                div { class: "rf-project-name", "{p.name}" }
                                div { class: "rf-project-desc",
                                    if p.description.is_empty() { "（无描述）" } else { "{p.description}" }
                                }
                                div { class: "rf-project-meta",
                                    span { "{count_map.get(&p.id).copied().unwrap_or(0)} 个接口" }
                                    span { "更新于 {format_time(&p.updated_at)}" }
                                    div { class: "spacer" }
                                    button {
                                        class: "rf-btn rf-btn-ghost rf-btn-sm",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            let mut cd = confirm_del;
                                            cd.set(Some((
                                                ConfirmInfo::new(
                                                    "删除项目",
                                                    format!("确定要删除项目「{p_name}」吗？其下的所有接口、环境、Mock 规则将一并删除，且不可恢复。"),
                                                ),
                                                pid,
                                            )));
                                        },
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "rf-divider" }
                div { class: "rf-form-row",
                    input {
                        class: "rf-input",
                        placeholder: "项目名称",
                        value: "{new_name}",
                        oninput: move |e| new_name.set(e.data().value()),
                    }
                    input {
                        class: "rf-input",
                        placeholder: "描述（可选）",
                        value: "{new_desc}",
                        oninput: move |e| new_desc.set(e.data().value()),
                    }
                    button {
                        class: "rf-btn rf-btn-primary",
                        onclick: move |_| {
                            let n = new_name.peek().clone();
                            let d = new_desc.peek().clone();
                            if !n.trim().is_empty() {
                                new_name.set(String::new());
                                new_desc.set(String::new());
                            }
                            match state.create_project(n, d) {
                                Ok(()) => {}
                                Err(msg) => state.toast_error(msg),
                            }
                        },
                        PlusIcon {}
                        "创建项目"
                    }
                }
                div { class: "rf-hint",
                    span { "点击项目卡片进入工作区" }
                    span { "·" }
                    kbd { class: "rf-kbd", "Ctrl/⌘+Enter" }
                    span { "发送" }
                    span { "·" }
                    kbd { class: "rf-kbd", "Ctrl/⌘+S" }
                    span { "保存" }
                    span { "·" }
                    kbd { class: "rf-kbd", "Ctrl/⌘+N" }
                    span { "新建接口" }
                    span { "·" }
                    kbd { class: "rf-kbd", "Ctrl/⌘+F" }
                    span { "搜索" }
                }
            }
            if let Some((info, _)) = confirm_del.read().as_ref() {
            ConfirmDialog {
                info: Some(info.clone()),
                on_confirm: move |_| {
                    if let Some((_, id)) = confirm_del.peek().as_ref() {
                        st_del.delete_project(*id);
                    }
                    let mut c = confirm_del;
                    c.set(None);
                },
                on_cancel: move |_| {
                    let mut c = confirm_del;
                    c.set(None);
                },
            }
        }
        }
    }
}
