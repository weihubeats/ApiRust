//! 工作区编辑器渲染测试（无头渲染）：
//! 验证「新建接口」后编辑器能正常展示方法选择、路径、Body 等编辑控件。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{AttributeValue, ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use fox_core::model::{Endpoint, EndpointStatus, HttpMethod, Project, RequestSpec};
    use sqlx::SqlitePool;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use uuid::Uuid;

    use crate::pages::workspace::WorkspacePage;
    use crate::services::Services;
    use crate::state::{AppState, Page};

    /// 根组件：在 VirtualDom 作用域内创建状态（Signal 必须挂载到作用域）。
    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let mut state = AppState::new(Services::new(pool));
        let project_id = Uuid::new_v4();
        state.projects.write().push(Project {
            id: project_id,
            name: "测试项目".into(),
            description: String::new(),
            variables: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        state.set_current_project(Some(project_id));
        // 模拟 create_endpoint_at 成功后的状态：接口已入库并在列表。
        let ep = Endpoint {
            id: Uuid::new_v4(),
            project_id,
            folder_id: None,
            name: "新建接口".into(),
            method: HttpMethod::GET,
            path: "/".into(),
            description: String::new(),
            status: EndpointStatus::Developing,
            sort_order: 0,
            request: RequestSpec::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        state.endpoints.write().push(ep.clone());
        state.current_page.set(Page::Workspace);
        // 模拟 create_endpoint_at 中 open_endpoint_tab 的效果。
        state.open_endpoint_tab(ep.id);
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        use_context_provider(move || state);
        rsx! { WorkspacePage {} }
    }

    thread_local! {
        static POOL: RefCell<Option<SqlitePool>> = const { RefCell::new(None) };
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
    }

    /// 所有动态文本节点（模板内的静态文本不会出现在 mutation 中；
    /// 模板实例化时的动态文本以 HydrateText / CreateTextNode 出现，后续更新为 SetText）。
    fn all_text(muts: &Mutations) -> Vec<String> {
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

    /// 编辑器主体（url-bar / tabs / tab-body）的模板源码行号起点。
    const EDITOR_TEMPLATE_START_LINE: usize = 1782;

    /// 收集渲染出的模板源码行号，用于识别编辑器主体是否渲染。
    fn template_lines(muts: &Mutations) -> Vec<usize> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::LoadTemplate { name, .. } = m {
                if let Some(first) = name.split(':').find_map(|s| s.parse::<usize>().ok()) {
                    out.push(first);
                }
            }
        }
        out
    }

    /// 收集动态属性（如 input 的 value）。
    fn attr_values(muts: &Mutations, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::SetAttribute { name: n, value, .. } = m {
                if *n == name {
                    if let AttributeValue::Text(t) = value {
                        out.push(t.to_string());
                    }
                }
            }
        }
        out
    }

    #[test]
    fn new_endpoint_editor_renders_full_controls() {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let pool = fox_storage::db::memory_pool().await.unwrap();
            POOL.with(|s| *s.borrow_mut() = Some(pool));

            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let m2 = dom.render_immediate_to_vec();
            let m3 = dom.render_immediate_to_vec();

            let mut texts = all_text(&m2);
            texts.extend(all_text(&m3));
            let mut lines = template_lines(&m1);
            lines.extend(template_lines(&m2));
            lines.extend(template_lines(&m3));
            let mut path_values = attr_values(&m2, "value");
            path_values.extend(attr_values(&m3, "value"));

            assert!(
                lines.iter().any(|l| *l >= EDITOR_TEMPLATE_START_LINE),
                "编辑器主体模板未渲染（应出现 url-bar/tabs/tab-body），模板行号：{lines:?}"
            );
            assert!(
                texts.iter().any(|t| t == "GET"),
                "应展示请求方法 GET（下拉框），实际文本：{texts:?}"
            );
            assert!(
                path_values.iter().any(|v| v == "/"),
                "路径输入框应显示默认路径 /，实际 value：{path_values:?}"
            );
            assert!(
                !texts.iter().any(|t| t.contains("未选择接口")),
                "不应停留在「未选择接口」空态"
            );
        });
    }

    /// 所有指定事件名（如 click / input）的监听元素 id（按挂载顺序）。
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

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedMouseData::default(),
        )))
    }

    fn form_input(value: &str) -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedFormData::new(value.to_string(), HashMap::new(), None),
        )))
    }

    #[test]
    fn curl_import_updates_editor_and_closes_modal() {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let pool = fox_storage::db::memory_pool().await.unwrap();
            POOL.with(|s| *s.borrow_mut() = Some(pool));

            let mut dom = VirtualDom::new_with_props(root, ());
            let _m1 = dom.rebuild_to_vec();
            let m2 = dom.render_immediate_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");

            // url-bar 的「导入 cURL」是第二个 click 监听（dropdown 之后）。
            let clicks1 = event_listeners(&m2, "click");
            let inputs1 = event_listeners(&m2, "input");
            let import_btn = clicks1[1];
            dom.handle_event("click", mouse(), import_btn, true);
            let m3 = dom.render_immediate_to_vec();
            let clicks3 = event_listeners(&m3, "click");
            let modal_ids: Vec<ElementId> = clicks3
                .iter()
                .filter(|id| !clicks1.contains(id))
                .copied()
                .collect();
            assert_eq!(modal_ids.len(), 4, "应出现导入弹窗（backdrop+弹窗体+取消+解析并导入）");

            // 弹窗 textarea 是新增的 input 监听。
            let inputs3 = event_listeners(&m3, "input");
            let textarea_ids: Vec<ElementId> = inputs3
                .iter()
                .filter(|id| !inputs1.contains(id))
                .copied()
                .collect();
            assert_eq!(textarea_ids.len(), 1, "弹窗应有一个 textarea");
            dom.handle_event(
                "input",
                form_input(r#"curl -X POST -H "Content-Type: application/json" -d '{"a":1}' https://api.example.com/users"#),
                textarea_ids[0],
                true,
            );
            let m4 = dom.render_immediate_to_vec();

            // 点「解析并导入」（弹窗新增监听中的最后一个）。
            dom.handle_event("click", mouse(), modal_ids[3], true);
            let m5 = dom.render_immediate_to_vec();

            // 弹窗应关闭：m5 相对 m1 不再有新增 click 监听。
            let clicks5 = event_listeners(&m5, "click");
            let fresh: Vec<ElementId> = clicks5
                .iter()
                .filter(|id| !clicks1.contains(id))
                .copied()
                .collect();
            assert!(fresh.is_empty(), "导入后弹窗应关闭，残留监听：{fresh:?}");

            // 成功提示 toast。
            let toasts: Vec<String> = state
                .toasts
                .read()
                .iter()
                .map(|t| t.message.clone())
                .collect();
            assert!(
                toasts.iter().any(|t| t.contains("导入成功")),
                "应提示导入成功，实际 toasts：{toasts:?}"
            );

            // 编辑器字段已更新：方法 POST + 路径 URL + 名称。
            let mut texts = all_text(&m4);
            texts.extend(all_text(&m5));
            let mut values = attr_values(&m4, "value");
            values.extend(attr_values(&m5, "value"));
            assert!(
                texts.iter().any(|t| t == "POST"),
                "方法应更新为 POST，实际文本：{texts:?}"
            );
            assert!(
                values.iter().any(|v| v == "https://api.example.com/users"),
                "路径应更新为 cURL URL，实际 value：{values:?}"
            );
            assert!(
                values.iter().any(|v| v == "users"),
                "接口名应从 URL 推断为 users，实际 value：{values:?}"
            );
        });
    }
}