//! 设置页：环境管理（M6）+ 项目变量（M6）+ 后续里程碑扩展。

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::components::dropdown::Dropdown;
use crate::components::icons::XIcon;
use crate::state::{AppState, Page};
use fox_core::model::Environment;
use fox_openapi::import::ConflictStrategy;
use uuid::Uuid;

/// 环境编辑草稿（名称 + 变量行）。
#[derive(Clone, PartialEq)]
struct EnvDraft {
    name: String,
    vars: Vec<(String, String)>,
}

/// 变量行 -> 变量 Map（跳过空键；空值保留以便清空）。
fn vars_from_rows(rows: &[(String, String)]) -> HashMap<String, String> {
    rows.iter()
        .filter(|(k, _)| !k.trim().is_empty())
        .map(|(k, v)| (k.trim().to_string(), v.clone()))
        .collect()
}

/// 变量行渲染：Key / Value 输入 + 删除。
fn var_row(draft: Signal<Option<EnvDraft>>, index: usize, key: String, value: String) -> Element {
    let mut d = draft;
    rsx! {
        div { class: "row",
            input {
                class: "rf-input grow",
                placeholder: "变量名",
                value: "{key}",
                oninput: move |e| {
                    let mut guard = d.write();
                    if let Some(dr) = guard.as_mut() {
                        if let Some(row) = dr.vars.get_mut(index) {
                            row.0 = e.data().value();
                        }
                    }
                },
            }
            input {
                class: "rf-input grow",
                placeholder: "变量值",
                value: "{value}",
                oninput: move |e| {
                    let mut guard = d.write();
                    if let Some(dr) = guard.as_mut() {
                        if let Some(row) = dr.vars.get_mut(index) {
                            row.1 = e.data().value();
                        }
                    }
                },
            }
            button {
                class: "rf-btn rf-btn-sm",
                onclick: move |_| {
                    let mut guard = d.write();
                    if let Some(dr) = guard.as_mut() {
                        if index < dr.vars.len() {
                            dr.vars.remove(index);
                        }
                    }
                },
                "删除"
            }
        }
    }
}

/// 单个环境条目。
fn env_item(
    env: &Environment,
    state: AppState,
    current_env_id: Option<Uuid>,
    editing: bool,
    draft: Option<EnvDraft>,
    mut env_draft: Signal<Option<EnvDraft>>,
    edit_id: Signal<Option<Uuid>>,
) -> Element {
    let env_id = env.id;
    let env_project_id = env.project_id;
    let env_name = env.name.clone();
    let env_vars = env.variables.clone();
    let env_created_at = env.created_at;
    let env_updated_at = env.updated_at;
    let env_name_display = env.name.clone();
    let select_btn = state.clone();
    let delete_btn = state.clone();
    let save_btn = state.clone();

    rsx! {
        div { class: "env-item",
            div { class: "row",
                span { class: "env-name", "{env_name_display}" }
                if current_env_id == Some(env_id) {
                    span { class: "env-current", "当前" }
                }
                div { class: "spacer" }
                button {
                    class: "rf-btn rf-btn-sm",
                    onclick: move |_| {
                        select_btn.select_environment(Some(env_id));
                        select_btn.toast_info(format!("已切换到环境「{env_name_display}」"));
                    },
                    "使用"
                }
                button {
                    class: "rf-btn rf-btn-sm",
                    onclick: move |_| {
                        let mut ei = edit_id;
                        ei.set(Some(env_id));
                        let mut ed = env_draft;
                        ed.set(Some(EnvDraft {
                            name: env_name.clone(),
                            vars: env_vars.clone().into_iter().collect(),
                        }));
                    },
                    "编辑"
                }
                button {
                    class: "rf-btn rf-btn-sm",
                    onclick: move |_| delete_btn.delete_environment(env_id),
                    "删除"
                }
            }
            if editing {
                if let Some(d) = &draft {
                    div { class: "env-editor",
                        div { class: "row",
                            input {
                                class: "rf-input grow",
                                placeholder: "环境名称",
                                value: "{d.name}",
                                oninput: move |e| {
                                    let mut guard = env_draft.write();
                                    if let Some(dr) = guard.as_mut() {
                                        dr.name = e.data().value();
                                    }
                                },
                            }
                        }
                        div { class: "kv-title", "变量（请求中通过 {{key}} 引用）" }
                        for (i, (k, v)) in d.vars.clone().into_iter().enumerate() {
                            { var_row(env_draft, i, k, v) }
                        }
                        div { class: "row",
                            button {
                                class: "rf-btn rf-btn-sm",
                                onclick: move |_| {
                                    let mut guard = env_draft.write();
                                    if let Some(dr) = guard.as_mut() {
                                        dr.vars.push((String::new(), String::new()));
                                    }
                                },
                                "添加变量"
                            }
                        button {
                            class: "rf-btn rf-btn-sm rf-btn-primary",
                            onclick: move |_| {
                                let name;
                                let vars;
                                {
                                    let mut guard = env_draft.write();
                                    let Some(dr) = guard.as_mut() else { return };
                                    name = dr.name.trim().to_string();
                                    vars = vars_from_rows(&dr.vars);
                                }
                                if name.is_empty() {
                                    save_btn.toast_error("环境名称不能为空");
                                    return;
                                }
                                let updated = Environment {
                                    id: env_id,
                                    project_id: env_project_id,
                                    name,
                                    variables: vars,
                                    created_at: env_created_at,
                                    updated_at: env_updated_at,
                                };
                                save_btn.save_environment(updated);
                                let mut ei = edit_id;
                                ei.set(None);
                                let mut ed = env_draft;
                                ed.set(None);
                            },
                            "保存环境"
                        }
                            button {
                                class: "rf-btn rf-btn-sm",
                                onclick: move |_| {
                                    let mut ei = edit_id;
                                    ei.set(None);
                                    let mut ed = env_draft;
                                    ed.set(None);
                                },
                                "取消"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let state = use_context::<AppState>();

    let mut new_env_name: Signal<String> = use_signal(String::new);
    let edit_id: Signal<Option<Uuid>> = use_signal(|| None);
    let env_draft: Signal<Option<EnvDraft>> = use_signal(|| None);
    let proj_vars: Signal<Vec<(String, String)>> = use_signal(Vec::new);
    let proj_loaded: Signal<Option<Uuid>> = use_signal(|| None);
    let mut openapi_text: Signal<String> = use_signal(String::new);
    let mut strategy: Signal<String> = use_signal(String::new);
    let mut mock_name: Signal<String> = use_signal(String::new);
    let mut mock_method: Signal<String> = use_signal(String::new);
    let mut mock_path: Signal<String> = use_signal(String::new);
    let mut mock_status_code: Signal<String> = use_signal(String::new);
    let mut mock_priority: Signal<String> = use_signal(String::new);
    let mut mock_delay: Signal<String> = use_signal(String::new);
    let mut mock_query: Signal<String> = use_signal(String::new);
    let mut mock_headers: Signal<String> = use_signal(String::new);
    let mut mock_body: Signal<String> = use_signal(String::new);
    let mut backup_text: Signal<String> = use_signal(String::new);

    {
        let st = state.clone();
        let mut pv = proj_vars;
        let mut loaded = proj_loaded;
        let mut ei = edit_id;
        let mut ed = env_draft;
        use_effect(move || {
            let current = *st.current_project_id.read();
            if *loaded.peek() == current {
                return;
            }
            loaded.set(current);
            let project = st.projects.read().clone();
            match current.and_then(|id| project.into_iter().find(|p| p.id == id)) {
                Some(p) => pv.set(p.variables.into_iter().collect()),
                None => pv.set(Vec::new()),
            }
            ei.set(None);
            ed.set(None);
        });
    }

    let current_env_id = *state.current_environment_id.peek();
    let env_list = state.environments.read().clone();
    let create_btn = state.clone();
    let save_proj_btn = state.clone();
    let import_btn = state.clone();
    let export_btn = state.clone();
    let back_btn = state.clone();
    let start_btn = state.clone();
    let stop_btn = state.clone();
    let add_rule_btn = state.clone();
    let backup_btn = state.clone();
    let restore_btn = state.clone();
    let check_btn = state.clone();
    let view_btn = state.clone();
    let close_btn = state.clone();
    let update_info = state.update_info.read().clone();
    let update_checking = *state.update_checking.read();
    let project_name = state
        .projects
        .read()
        .iter()
        .find(|p| Some(p.id) == *state.current_project_id.peek())
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "未选择项目".into());
    let editing = *edit_id.peek();
    let draft = env_draft.peek().clone();
    let proj_rows = proj_vars.read().clone();
    let mock_running = state.mock_running();
    let mock_addr = state
        .mock_port
        .peek()
        .map(|p| format!("http://127.0.0.1:{p}"))
        .unwrap_or_default();
    let mock_rule_list = state.mock_rules.read().clone();
    type MockRow = (Uuid, String, String, String, u16, i64, u64, AppState);
    let mock_rows: Vec<MockRow> = mock_rule_list
        .iter()
        .map(|r| {
            (
                r.id,
                r.method.as_str().to_string(),
                r.method.as_str().to_lowercase(),
                r.path.clone(),
                r.response_status,
                r.priority,
                r.delay_ms,
                state.clone(),
            )
        })
        .collect();

    rsx! {
        div {
            class: "page-pad",
            // 点击设置内容外的空白区域关闭设置页。
            onclick: move |_| {
                let mut pg = close_btn.current_page;
                pg.set(Page::Workspace);
            },
            div {
                class: "center-box",
                // 设置内容区不冒泡到空白区关闭逻辑。
                onclick: |e| { e.stop_propagation(); },
                div { class: "settings-header",
                    h2 { "设置" }
                    button {
                        class: "rf-btn rf-btn-ghost rf-btn-sm settings-close",
                        title: "关闭设置",
                        onclick: move |_| {
                            let mut pg = close_btn.current_page;
                            pg.set(Page::Workspace);
                        },
                        XIcon {}
                    }
                }
                div { class: "rf-divider" }
                div { class: "hint rf-hint-flat",
                    "当前项目：{project_name}。变量优先级：环境 > 项目 > 内置（{{$uuid}} / {{$timestamp}} / {{$isoTimestamp}} / {{$randomInt}}）。" }
                div { class: "settings-section",
                    div { class: "section-title", "环境管理" }
                    div { class: "row",
                        input {
                            class: "rf-input grow",
                            placeholder: "新环境名称，如：生产 / 测试",
                            value: "{new_env_name}",
                            oninput: move |e| new_env_name.set(e.data().value()),
                        }
                        button {
                            class: "rf-btn rf-btn-primary",
                            onclick: move |_| {
                                let name = new_env_name.peek().trim().to_string();
                                if name.is_empty() {
                                    create_btn.toast_error("环境名称不能为空");
                                    return;
                                }
                                create_btn.create_environment(name);
                                new_env_name.set(String::new());
                            },
                            "新建环境"
                        }
                    }
                    if env_list.is_empty() {
                        div { class: "hint", "暂无环境。新建环境后可在顶部选择器切换，发送请求时将自动替换 {{base_url}} 等变量。" }
                    }
                    for env in env_list {
                        { env_item(&env, state.clone(), current_env_id, editing == Some(env.id), draft.clone(), env_draft, edit_id) }
                    }
                }
                div { class: "rf-divider" }
                div { class: "settings-section",
                    div { class: "section-title", "项目变量" }
                    div { class: "hint", "项目变量对所有环境生效，环境变量同名时覆盖项目变量。" }
                    for (i, (k, v)) in proj_rows.clone().into_iter().enumerate() {
                        { proj_var_row(proj_vars, i, k, v) }
                    }
                    div { class: "row",
                        button {
                            class: "rf-btn rf-btn-sm",
                            onclick: move |_| {
                                let mut pv = proj_vars;
                                pv.write().push((String::new(), String::new()));
                            },
                            "添加变量"
                        }
                        button {
                            class: "rf-btn rf-btn-sm rf-btn-primary",
                            onclick: move |_| {
                                let rows = proj_vars.read().clone();
                                save_proj_btn.save_project_variables(vars_from_rows(&rows));
                            },
                            "保存项目变量"
                        }
                    }
                }
                    div { class: "rf-divider" }
                    div { class: "settings-section",
                        div { class: "section-title", "OpenAPI 导入导出" }
                        div { class: "hint", "粘贴 OpenAPI 3.0 / Swagger 2.0 / Postman 集合 v2.1（JSON 或 YAML）内容，自动识别格式导入；或导出当前项目为 OpenAPI 3.0 JSON。" }
                        textarea {
                            class: "rf-textarea rf-oapi-input",
                            placeholder: "在此粘贴 OpenAPI JSON / YAML 内容…",
                            value: "{openapi_text}",
                            oninput: move |e| openapi_text.set(e.data().value()),
                        }
                        div { class: "row",
                            span { class: "label-hint", "冲突策略：" }
                            Dropdown {
                                options: vec![
                                    ("skip".into(), "跳过重复".into()),
                                    ("overwrite".into(), "覆盖重复".into()),
                                    ("duplicate".into(), "复制为新接口".into()),
                                ],
                                selected: {
                                    let s = strategy.peek().clone();
                                    if s.is_empty() { "skip".into() } else { s }
                                },
                                on_select: move |v: String| strategy.set(v),
                            }
                            div { class: "spacer" }
                            button {
                                class: "rf-btn rf-btn-primary",
                                onclick: move |_| {
                                    let text = openapi_text.peek().clone();
                                    if text.trim().is_empty() {
                                        import_btn.toast_error("请先粘贴 OpenAPI 内容");
                                        return;
                                    }
                                    let strategy =
                                        ConflictStrategy::from_str_cn(strategy.peek().as_str())
                                            .unwrap_or_default();
                                    import_btn.import_openapi(text, strategy);
                                },
                                "导入"
                            }
                            button {
                                class: "rf-btn",
                                onclick: move |_| {
                                    let exporter = export_btn.clone();
                                    export_btn.export_openapi(move |result| {
                                        let mut t = openapi_text;
                                        match result {
                                            Ok(json) => {
                                                t.set(json);
                                                exporter.toast_success("导出完成，内容已填入文本框（可复制保存）");
                                            }
                                            Err(e) => exporter.toast_error(e),
                                        }
                                    });
                                },
                                "导出当前项目"
                            }
                        }
                    }
                    div { class: "rf-divider" }
                    div { class: "settings-section",
                        div { class: "section-title", "Mock Server" }
                        div { class: "row",
                            if mock_running {
                                span { class: "mock-status ok", "运行中" }
                                span { class: "env-current", "{mock_addr}" }
                                button {
                                    class: "rf-btn rf-btn-sm",
                                    onclick: move |_| stop_btn.stop_mock(),
                                    "停止"
                                }
                                div { class: "hint-inline", "自定义规则 / 接口改动后重启才会生效。" }
                            } else {
                                span { class: "mock-status off", "未启动" }
                                button {
                                    class: "rf-btn rf-btn-primary",
                                    onclick: move |_| start_btn.start_mock(),
                                    "启动 Mock"
                                }
                            }
                        }
                        div { class: "row rf-mt-2",
                            span { class: "label-hint", "地址：" }
                            span { class: "hint-inline", "http://127.0.0.1:4010（占用时自动 +1，最大尝试 20 次）" }
                        }
                        div { class: "kv-title", "自定义 Mock 规则（优先级高于接口响应示例）" }
                        div { class: "row",
                            input {
                                class: "rf-input grow",
                                placeholder: "规则名称",
                                value: "{mock_name}",
                                oninput: move |e| mock_name.set(e.data().value()),
                            }
                            Dropdown {
                                class: "rf-dd-method",
                                options: vec![
                                    ("GET".into(), "GET".into()),
                                    ("POST".into(), "POST".into()),
                                    ("PUT".into(), "PUT".into()),
                                    ("DELETE".into(), "DELETE".into()),
                                    ("PATCH".into(), "PATCH".into()),
                                ],
                                selected: {
                                    let s = mock_method.peek().clone();
                                    if s.is_empty() { "GET".into() } else { s }
                                },
                                on_select: move |v: String| mock_method.set(v),
                            }
                            input {
                                class: "rf-input grow",
                                placeholder: "路径，如 /users/100",
                                value: "{mock_path}",
                                oninput: move |e| mock_path.set(e.data().value()),
                            }
                        }
                        div { class: "row",
                            input {
                                class: "rf-input rf-input-sm rf-in-short",
                                placeholder: "状态码",
                                value: "{mock_status_code}",
                                oninput: move |e| mock_status_code.set(e.data().value()),
                            }
                            input {
                                class: "rf-input rf-input-sm rf-in-short",
                                placeholder: "优先级",
                                value: "{mock_priority}",
                                oninput: move |e| mock_priority.set(e.data().value()),
                            }
                            input {
                                class: "rf-input rf-input-sm rf-in-short",
                                placeholder: "延迟 ms",
                                value: "{mock_delay}",
                                oninput: move |e| mock_delay.set(e.data().value()),
                            }
                        }
                        div { class: "row",
                            textarea {
                                class: "rf-textarea rf-mock-field",
                                rows: 1,
                                placeholder: "Query 匹配：每行 key=value（value 留空表示 key 存在即可）",
                                value: "{mock_query}",
                                oninput: move |e| mock_query.set(e.data().value()),
                            }
                            textarea {
                                class: "rf-textarea rf-mock-field",
                                rows: 1,
                                placeholder: "Header 匹配：每行 key=value",
                                value: "{mock_headers}",
                                oninput: move |e| mock_headers.set(e.data().value()),
                            }
                        }
                        textarea {
                            class: "rf-textarea rf-mock-body",
                            rows: 3,
                            placeholder: "响应 body 模板，支持 {{mock.email}} / {{params.id}} / {{query.name}} / {{headers.X}} 等变量",
                            value: "{mock_body}",
                            oninput: move |e| mock_body.set(e.data().value()),
                        }
                        div { class: "row",
                            button {
                                class: "rf-btn rf-btn-sm rf-btn-primary",
                                onclick: move |_| {
                                    add_rule_btn.add_mock_rule(
                                        mock_name.peek().clone(),
                                        mock_method.peek().clone(),
                                        mock_path.peek().clone(),
                                        mock_status_code.peek().clone(),
                                        mock_priority.peek().clone(),
                                        mock_delay.peek().clone(),
                                        mock_query.peek().clone(),
                                        mock_headers.peek().clone(),
                                        mock_body.peek().clone(),
                                    );
                                },
                                "新建规则"
                            }
                        }
                        for (id, method, method_cls, path, response_status, priority, delay_ms, row_state) in mock_rows.clone() {
                            div { class: "mock-rule-row",
                                div { class: "row",
                                    span { class: "rf-method rf-method-chip rf-method-chip-{method_cls}", "{method}" }
                                    span { class: "url", "{path}" }
                                    span { class: "hint-inline", "状态 {response_status} · 优先级 {priority} · 延迟 {delay_ms}ms" }
                                    div { class: "spacer" }
                                    button {
                                        class: "rf-btn rf-btn-sm",
                                        onclick: move |_| row_state.delete_mock_rule(id),
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "rf-divider" }
                    div { class: "settings-section",
                        div { class: "section-title", "备份 / 恢复" }
                        div { class: "row",
                            button {
                                class: "rf-btn rf-btn-primary",
                                onclick: move |_| backup_btn.backup_project(),
                                "备份当前项目"
                            }
                            span { class: "hint-inline", "导出为 JSON 文件（含接口、环境、Mock 规则、响应示例），保存到 ~/.rustfox/backups/" }
                        }
                        div { class: "backup-box",
                            span { class: "label-hint", "恢复备份（粘贴 JSON，将创建为全新项目，不覆盖现有数据）：" }
                            textarea {
                                class: "rf-textarea",
                                placeholder: "在此粘贴备份 JSON 内容",
                                value: "{backup_text}",
                                oninput: move |e| backup_text.set(e.data().value()),
                            }
                            div { class: "row rf-mt-2",
                                button {
                                    class: "rf-btn rf-btn-sm rf-btn-primary",
                                    onclick: move |_| {
                                        restore_btn.restore_backup(backup_text.peek().clone());
                                    },
                                    "恢复"
                                }
                            }
                        }
                    }
                    div { class: "rf-divider" }
                    div { class: "settings-section",
                        div { class: "section-title", "关于" }
                        { about_rows() }
                        div { class: "row",
                            button {
                                class: "rf-btn rf-btn-primary",
                                onclick: move |_| check_btn.check_for_update(true),
                                "检查更新",
                            }
                            if let Some(info) = &update_info {
                                span { class: "env-current", "发现新版本 v{info.version}" }
                                button {
                                    class: "rf-btn",
                                    onclick: move |_| view_btn.open_update_modal(),
                                    "查看更新",
                                }
                            } else if update_checking {
                                span { class: "hint-inline", "正在检查更新…" }
                            }
                        }
                    }
                div { class: "row rf-mt-4",
                    button {
                        class: "rf-btn",
                        onclick: move |_| { let mut pg = back_btn.current_page; pg.set(Page::Home); },
                        "返回首页"
                    }
                }
            }
        }
    }
}

/// 项目变量行渲染。
fn proj_var_row(
    draft: Signal<Vec<(String, String)>>,
    index: usize,
    key: String,
    value: String,
) -> Element {
    let mut d = draft;
    rsx! {
        div { class: "row",
            input {
                class: "rf-input grow",
                placeholder: "变量名",
                value: "{key}",
                oninput: move |e| {
                    let mut guard = d.write();
                    if let Some(row) = guard.get_mut(index) {
                        row.0 = e.data().value();
                    }
                },
            }
            input {
                class: "rf-input grow",
                placeholder: "变量值",
                value: "{value}",
                oninput: move |e| {
                    let mut guard = d.write();
                    if let Some(row) = guard.get_mut(index) {
                        row.1 = e.data().value();
                    }
                },
            }
            button {
                class: "rf-btn rf-btn-sm",
                onclick: move |_| {
                    let mut guard = d.write();
                    if index < guard.len() {
                        guard.remove(index);
                    }
                },
                "删除"
            }
        }
    }
}

/// 「关于」区信息行（应用版本、平台、数据目录等）。
fn about_rows() -> Element {
    let rows = crate::updater::about_meta();
    rsx! {
        div { class: "about-table",
            for (k, v) in rows {
                div { class: "row",
                    span { class: "label-hint", "{k}" }
                    span { class: "about-value", "{v}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::vars_from_rows;

    #[test]
    fn vars_skip_empty_keys_keep_values() {
        let rows = vec![
            ("base_url".into(), "https://api.example.com".into()),
            ("".into(), "应被跳过".into()),
            ("  ".into(), "空白键也跳过".into()),
            ("token".into(), String::new()),
        ];
        let map = vars_from_rows(&rows);
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("base_url").map(String::as_str),
            Some("https://api.example.com")
        );
        assert_eq!(map.get("token").map(String::as_str), Some(""));
        assert!(!map.contains_key(""));
    }

    #[test]
    fn vars_keys_are_trimmed() {
        let rows = vec![("  base_url  ".into(), "v".into())];
        let map = vars_from_rows(&rows);
        assert_eq!(map.get("base_url").map(String::as_str), Some("v"));
    }

    #[test]
    fn vars_empty_input_yields_empty_map() {
        let map = vars_from_rows(&[]);
        assert!(map.is_empty());
    }
}
