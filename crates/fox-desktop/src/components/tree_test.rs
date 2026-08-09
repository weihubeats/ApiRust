//! 左侧目录树行为测试（无头渲染 + 事件模拟）：验证「导入 cURL」能打开导入弹窗。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{AttributeValue, ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use fox_core::model::{Endpoint, EndpointStatus, HttpMethod, Project, RequestSpec};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use uuid::Uuid;

    use crate::components::project_tree::SideBar;
    use crate::services::Services;
    use crate::state::AppState;

    /// 根组件：在 VirtualDom 作用域内创建状态（Signal 必须挂载到作用域）。
    fn root(_: ()) -> Element {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(fox_storage::db::memory_pool()).unwrap();
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
        state.endpoints.write().push(Endpoint {
            id: Uuid::new_v4(),
            project_id,
            folder_id: None,
            name: "常用列表".into(),
            method: HttpMethod::GET,
            path: "/api/users".into(),
            description: String::new(),
            status: EndpointStatus::Developing,
            sort_order: 0,
            request: RequestSpec::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        use_context_provider(move || state);
        rsx! { SideBar {} }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
    }

    /// 所有 click 监听元素 id（按挂载顺序）。
    fn click_listeners(muts: &Mutations) -> Vec<ElementId> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::NewEventListener { name, id, .. } = m {
                if name == "click" {
                    out.push(*id);
                }
            }
        }
        out
    }

    /// 指定 attr id（如 import-curl-toolbar）元素的 click 监听元素 id。
    fn listener_id_for(muts: &Mutations, attr_id: &str) -> Vec<ElementId> {
        let mut elems = Vec::new();
        for m in &muts.edits {
            if let Mutation::SetAttribute { name, value, id, .. } = m {
                if *name == "id" && value == &AttributeValue::Text(attr_id.to_string()) {
                    elems.push(*id);
                }
            }
        }
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::NewEventListener { name, id, .. } = m {
                if name == "click" && elems.contains(id) {
                    out.push(*id);
                }
            }
        }
        out
    }

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        )))
    }

    fn with_converter(f: impl FnOnce()) {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        f();
    }

    #[test]
    fn import_curl_opens_and_closes_modal() {
        with_converter(|| {
            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
            let ep_id = state.endpoints.read()[0].id;

            // 初始不应有弹窗（无 backdrop / 弹窗按钮监听）。
            let before: Vec<ElementId> = click_listeners(&m1);
            assert_eq!(before.len(), 8, "初始监听：3 个顶栏按钮 + 接口行 4 按钮 + 树行 open");

            // 顶栏「导入 cURL」可点开弹窗：新增 backdrop + 弹窗体 + 取消 + 解析并导入 4 个监听。
            // 顶栏按钮是第三个 click 监听（＋ 文件夹、＋ 接口、导入 cURL）。
            let toolbar_btn = before[2];
            dom.handle_event("click", mouse(), toolbar_btn, true);
            let m2 = dom.render_immediate_to_vec();
            let after_open = click_listeners(&m2);
            let fresh: Vec<ElementId> = after_open
                .iter()
                .filter(|id| !before.contains(id))
                .copied()
                .collect();
            assert_eq!(
                fresh.len(),
                4,
                "点击后应出现导入弹窗（backdrop+弹窗体+取消+解析并导入）"
            );

            // 点 backdrop 关闭。
            dom.handle_event("click", mouse(), fresh[0], true);
            let m3 = dom.render_immediate_to_vec();
            let after_close = click_listeners(&m3);
            let fresh_close: Vec<ElementId> = after_close
                .iter()
                .filter(|id| !before.contains(id))
                .copied()
                .collect();
            assert!(fresh_close.is_empty(), "backdrop 点击应关闭弹窗");

            // 接口行的「导入cURL」同样能打开弹窗（元素 id 在 m1 中注册，渲染间保持）。
            let row = listener_id_for(&m1, &format!("import-curl-{}", ep_id));
            assert_eq!(row.len(), 1, "接口行应有「导入cURL」按钮");
            dom.handle_event("click", mouse(), row[0], true);
            let m4 = dom.render_immediate_to_vec();
            let after_reopen = click_listeners(&m4);
            let fresh2: Vec<ElementId> = after_reopen
                .iter()
                .filter(|id| !before.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh2.len(), 4, "接口行按钮也应弹出导入弹窗");
        });
    }
}