//! 工作区空状态视图：未选择接口时替代中央区域的简单文字提示。
//!
//! 视觉：居中容器 + 大尺寸低透明度雷达 SVG + 主副标题 + 快捷操作按钮 + 快捷键小贴士。

use dioxus::prelude::*;
use fox_core::curl_parser::parse_curl;
use fox_openapi::import::ConflictStrategy;

use crate::state::AppState;

/// 雷达 / API 连接 SVG（单色，透明度由容器样式控制）。
fn radar_icon() -> Element {
    rsx! {
        svg {
            view_box: "0 0 64 64",
            circle { cx: "32", cy: "32", r: "28", fill: "none", stroke: "currentColor",
                stroke_width: "2.5", opacity: "0.9" }
            circle { cx: "32", cy: "32", r: "18", fill: "none", stroke: "currentColor",
                stroke_width: "2", opacity: "0.65" }
            circle { cx: "32", cy: "32", r: "8", fill: "none", stroke: "currentColor",
                stroke_width: "2", opacity: "0.4" }
            path { d: "M32 32 L64 32 A32 32 0 0 1 55.4 53.8 Z", fill: "currentColor",
                opacity: "0.28" }
            line { x1: "32", y1: "2", x2: "32", y2: "62", stroke: "currentColor",
                stroke_width: "1", opacity: "0.4", stroke_dasharray: "3 4" }
            line { x1: "2", y1: "32", x2: "62", y2: "32", stroke: "currentColor",
                stroke_width: "1", opacity: "0.4", stroke_dasharray: "3 4" }
            circle { cx: "45", cy: "19", r: "2.6", fill: "currentColor", opacity: "0.9" }
            circle { cx: "20", cy: "45", r: "2.2", fill: "currentColor", opacity: "0.7" }
            circle { cx: "33", cy: "50", r: "1.8", fill: "currentColor", opacity: "0.5" }
        }
    }
}

/// 工作区空状态：新建接口 / 导入 cURL / 导入 OpenAPI 三个快捷入口。
#[allow(non_snake_case)]
pub fn EmptyState() -> Element {
    let state = use_context::<AppState>();

    // 快捷操作弹窗的本地状态。
    let mut curl_open: Signal<bool> = use_signal(|| false);
    let mut curl_input: Signal<String> = use_signal(String::new);
    let mut oapi_open: Signal<bool> = use_signal(|| false);
    let mut oapi_text: Signal<String> = use_signal(String::new);

    // 主按钮：新建接口（默认名，可后续重命名）。
    let create_btn = state.clone();
    let new_endpoint = move || {
        create_btn.create_endpoint_at(None, "未命名接口".to_string());
    };

    // 从 cURL 解析并创建接口。
    let mut do_import_curl = {
        let st = state.clone();
        let mut co = curl_open;
        let mut ci = curl_input;
        move || {
            let raw = ci.peek().clone();
            match parse_curl(&raw) {
                Ok(parsed) => {
                    st.create_endpoint_from_curl(None, &parsed);
                    co.set(false);
                    ci.set(String::new());
                }
                Err(e) => st.toast_error(format!("cURL 格式无法识别：{}", e.user_message())),
            }
        }
    };

    // 导入 OpenAPI（跳过重复策略）。
    let mut do_import_openapi = {
        let st = state.clone();
        let mut oo = oapi_open;
        let mut ot = oapi_text;
        move || {
            let text = ot.peek().clone();
            if text.trim().is_empty() {
                st.toast_error("请先粘贴 OpenAPI 内容");
                return;
            }
            st.import_openapi(text, ConflictStrategy::Skip);
            oo.set(false);
            ot.set(String::new());
        }
    };

    let curl_open_flag = *curl_open.read();
    let oapi_open_flag = *oapi_open.read();

    rsx! {
        div { class: "ws-empty",
            div { class: "ws-empty-icon",
                {radar_icon()}
            }
            div { class: "ws-empty-title", "开始构建你的 API 工作流" }
            div { class: "ws-empty-sub", "创建新接口，或从 cURL / OpenAPI 快速导入" }
            div { class: "ws-empty-actions",
                button {
                    class: "rf-btn rf-btn-primary",
                    onclick: move |_| new_endpoint(),
                    "+ 新建接口",
                }
                button {
                    class: "rf-btn",
                    onclick: move |_| curl_open.set(true),
                    "导入 cURL",
                }
                button {
                    class: "rf-btn",
                    onclick: move |_| oapi_open.set(true),
                    "导入 OpenAPI",
                }
            }
            div { class: "ws-empty-tips",
                div { class: "ws-empty-tips-title", "小贴士" }
                div { class: "ws-empty-tips-row",
                    span { class: "ws-kbd", "Ctrl+N" } span { "新建接口" }
                    span { class: "ws-kbd", "Ctrl+K" } span { "搜索接口" }
                    span { class: "ws-kbd", "Ctrl+Enter" } span { "快速发送请求" }
                }
            }
        }
        if curl_open_flag {
            div {
                class: "modal-backdrop",
                onclick: move |_| curl_open.set(false),
                div {
                    class: "modal curl-modal",
                    onclick: |e| { e.stop_propagation(); },
                    h3 { "从 cURL 导入接口" }
                    div {
                        class: "hint",
                        "粘贴浏览器「Copy as cURL」复制的命令，自动解析方法、URL、请求头、Body 与认证，并创建接口。",
                    }
                    textarea {
                        class: "rf-textarea curl-input",
                        rows: "10",
                        placeholder: "curl -X POST https://api.example.com/users \\\n  -H \"Content-Type: application/json\" \\\n  -d \"{{\"name\":\"test\"}}\"",
                        value: "{curl_input}",
                        oninput: move |e| {
                            let v = e.data().value();
                            curl_input.set(v);
                        },
                    }
                    div { class: "rf-modal-actions",
                        button {
                            class: "rf-btn",
                            onclick: move |_| curl_open.set(false),
                            "取消",
                        }
                        button {
                            class: "rf-btn rf-btn-primary",
                            onclick: move |_| do_import_curl(),
                            "解析并导入",
                        }
                    }
                }
            }
        }
        if oapi_open_flag {
            div {
                class: "modal-backdrop",
                onclick: move |_| oapi_open.set(false),
                div {
                    class: "modal curl-modal",
                    onclick: |e| { e.stop_propagation(); },
                    h3 { "从 OpenAPI 导入接口" }
                    div {
                        class: "hint",
                        "粘贴 OpenAPI 3.0 / Swagger 2.0 / Postman 集合 v2.1（JSON 或 YAML），自动识别格式导入；重复接口默认跳过。",
                    }
                    textarea {
                        class: "rf-textarea curl-input",
                        rows: "10",
                        placeholder: "在此粘贴 OpenAPI JSON / YAML 内容…",
                        value: "{oapi_text}",
                        oninput: move |e| {
                            let v = e.data().value();
                            oapi_text.set(v);
                        },
                    }
                    div { class: "rf-modal-actions",
                        button {
                            class: "rf-btn",
                            onclick: move |_| oapi_open.set(false),
                            "取消",
                        }
                        button {
                            class: "rf-btn rf-btn-primary",
                            onclick: move |_| do_import_openapi(),
                            "导入",
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use fox_core::model::{HttpMethod, Project};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use uuid::Uuid;

    use crate::services::Services;
    use crate::state::AppState;

    use super::EmptyState;

    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let project_id = PROJECT.with(|s| s.borrow().clone()).expect("项目已就绪");
        let mut state = AppState::new(Services::new(pool.clone()));
        state.projects.write().push(Project {
            id: project_id,
            name: "测试项目".into(),
            description: String::new(),
            variables: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        state.set_current_project(Some(project_id));
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        use_context_provider(move || state);
        rsx! { EmptyState {} }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
        static PROJECT: RefCell<Option<Uuid>> = const { RefCell::new(None) };
    }

    fn with_pool(f: impl FnOnce()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let project_id = Uuid::new_v4();
            let pool = fox_storage::db::memory_pool().await.unwrap();
            let project = Project {
                id: project_id,
                name: "测试项目".into(),
                description: String::new(),
                variables: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            fox_storage::repository::save_project(&pool, &project)
                .await
                .unwrap();
            POOL.with(|s| *s.borrow_mut() = Some(pool));
            PROJECT.with(|s| *s.borrow_mut() = Some(project_id));
        });
        f();
    }

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        )))
    }

    fn form_input(value: &str) -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedFormData::new(value.to_string(), HashMap::new(), None),
        )))
    }

    fn event_listeners(muts: &Mutations, name: &str) -> Vec<ElementId> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::NewEventListener { name: n, id, .. } = m {
                if *n == name {
                    out.push(*id);
                }
            }
        }
        out
    }

    #[test]
    fn empty_state_renders_three_action_buttons_without_modals() {
        with_pool(|| {
            dioxus_html::set_event_converter(Box::new(
                dioxus_html::SerializedHtmlEventConverter,
            ));
            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            // 静态文案/类名随 LoadTemplate 内置，Mutation 中不可见；
            // 行为由下方三个按钮测试覆盖，这里仅验证「3 按钮 + 无弹窗」。
            assert_eq!(
                event_listeners(&m1, "click").len(),
                3,
                "应有 3 个快捷操作按钮，无弹窗监听"
            );
            assert!(
                event_listeners(&m1, "input").is_empty(),
                "初始不应有输入控件（弹窗未打开）"
            );
        });
    }

    #[test]
    fn new_endpoint_button_creates_endpoint() {
        with_pool(|| {
            dioxus_html::set_event_converter(Box::new(
                dioxus_html::SerializedHtmlEventConverter,
            ));
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut dom = VirtualDom::new_with_props(root, ());
                let m1 = dom.rebuild_to_vec();
                let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let clicks = event_listeners(&m1, "click");
                // 第 1 个按钮：+ 新建接口。
                dom.handle_event("click", mouse(), clicks[0], true);
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let _ = dom.render_immediate_to_vec();
                }
                let eps = state.endpoints.read().clone();
                assert_eq!(eps.len(), 1, "应创建 1 个接口");
                assert_eq!(eps[0].name, "未命名接口");
                assert!(state.active_endpoint_id.peek().is_some(), "应打开新接口标签");
            });
        });
    }

    #[test]
    fn curl_import_creates_endpoint_from_command() {
        with_pool(|| {
            dioxus_html::set_event_converter(Box::new(
                dioxus_html::SerializedHtmlEventConverter,
            ));
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut dom = VirtualDom::new_with_props(root, ());
                let m1 = dom.rebuild_to_vec();
                let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let clicks1 = event_listeners(&m1, "click");
                let inputs1 = event_listeners(&m1, "input");

                // 第 2 个按钮：导入 cURL → 弹窗。
                dom.handle_event("click", mouse(), clicks1[1], true);
                let m2 = dom.render_immediate_to_vec();
                let clicks2 = event_listeners(&m2, "click");
                let fresh: Vec<ElementId> = clicks2
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(fresh.len(), 4, "弹窗应为 backdrop+弹窗体+取消+解析并导入");

                // textarea 输入 cURL 命令。
                let inputs2 = event_listeners(&m2, "input");
                let textarea_ids: Vec<ElementId> = inputs2
                    .iter()
                    .filter(|id| !inputs1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(textarea_ids.len(), 1, "应有一个 textarea");
                dom.handle_event(
                    "input",
                    form_input(r#"curl -X POST -H "Content-Type: application/json" -d '{"a":1}' https://api.example.com/users"#),
                    textarea_ids[0],
                    true,
                );
                let _m3 = dom.render_immediate_to_vec();

                // 点「解析并导入」（新增监听的最后一个）。
                dom.handle_event("click", mouse(), fresh[3], true);
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let _ = dom.render_immediate_to_vec();
                }
                let eps = state.endpoints.read().clone();
                assert!(
                    eps.iter().any(|e| e.path == "https://api.example.com/users"
                        && e.method == HttpMethod::POST),
                    "应从 cURL 创建 POST 接口，实际：{:?}",
                    eps.iter().map(|e| (e.name.clone(), e.path.clone())).collect::<Vec<_>>()
                );
            });
        });
    }

    #[test]
    fn openapi_import_creates_endpoints_and_closes_modal() {
        with_pool(|| {
            dioxus_html::set_event_converter(Box::new(
                dioxus_html::SerializedHtmlEventConverter,
            ));
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut dom = VirtualDom::new_with_props(root, ());
                let m1 = dom.rebuild_to_vec();
                let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let clicks1 = event_listeners(&m1, "click");
                let inputs1 = event_listeners(&m1, "input");

                // 第 3 个按钮：导入 OpenAPI → 弹窗。
                dom.handle_event("click", mouse(), clicks1[2], true);
                let m2 = dom.render_immediate_to_vec();
                let clicks2 = event_listeners(&m2, "click");
                let fresh: Vec<ElementId> = clicks2
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(fresh.len(), 4, "弹窗应为 backdrop+弹窗体+取消+导入");

                let inputs2 = event_listeners(&m2, "input");
                let textarea_ids: Vec<ElementId> = inputs2
                    .iter()
                    .filter(|id| !inputs1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(textarea_ids.len(), 1, "应有一个 textarea");
                dom.handle_event(
                    "input",
                    form_input(r#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},"paths":{"/ping":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#),
                    textarea_ids[0],
                    true,
                );
                let _m3 = dom.render_immediate_to_vec();

                dom.handle_event("click", mouse(), fresh[3], true);
                for _ in 0..30 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let _ = dom.render_immediate_to_vec();
                }
                let eps = state.endpoints.read().clone();
                assert!(
                    eps.iter().any(|e| e.path == "/ping" && e.method == HttpMethod::GET),
                    "应从 OpenAPI 创建 GET /ping，实际：{:?}",
                    eps.iter().map(|e| (e.name.clone(), e.path.clone())).collect::<Vec<_>>()
                );
                let toasts: Vec<String> = state
                    .toasts
                    .read()
                    .iter()
                    .map(|t| t.message.clone())
                    .collect();
                assert!(
                    toasts.iter().any(|t| t.contains("导入完成")),
                    "应提示导入完成，实际：{toasts:?}"
                );
            });
        });
    }
}