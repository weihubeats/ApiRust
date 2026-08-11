//! 侧边栏行为测试（无头渲染 + 事件模拟）：
//! - 布局：Header 项目选择器 / Toolbar 双按钮 / Search / Tree / Footer 环境选择器；
//! - 「＋ 接口」弹名称弹窗并按名称建接口；接口行「导入cURL」弹窗导入并创建接口。

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
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let project_id = PROJECT.with(|s| s.borrow().clone()).expect("项目已就绪");
        let mut state = AppState::new(Services::new(pool.clone()));
        // 项目行在 setup 阶段已入库（外键约束），此处在状态中保持一致。
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
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
        static PROJECT: RefCell<Option<Uuid>> = const { RefCell::new(None) };
    }

    /// 创建内存连接池并写入项目行（SQLite 外键约束要求项目已存在），记录项目 id。
    fn setup_pool(rt: &tokio::runtime::Runtime) -> Uuid {
        let project_id = Uuid::new_v4();
        let pool = rt.block_on(async {
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
            pool
        });
        POOL.with(|s| *s.borrow_mut() = Some(pool));
        PROJECT.with(|s| *s.borrow_mut() = Some(project_id));
        project_id
    }

    /// 指定 attr id 元素的 click 监听元素 id。
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

    fn form_input(value: &str) -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedFormData::new(value.to_string(), HashMap::new(), None),
        )))
    }

    /// 特定事件名的监听元素 id（按挂载顺序）。
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

    fn with_converter(f: impl FnOnce()) {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        f();
    }

    /// 先准备内存连接池（SQLite :memory: 需在事件循环外创建），再执行测试体。
    fn with_pool(f: impl FnOnce()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = setup_pool(&rt);
        f();
    }

    #[test]
    fn sidebar_layout_and_modals() {
        with_pool(|| { with_converter(|| {
            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
            let ep_id = state.endpoints.read()[0].id;

            // 初始 click 监听：项目/环境下拉 trigger + Toolbar 双按钮 + 接口行 open + 4 行内按钮。
            let initial = event_listeners(&m1, "click");
            assert_eq!(initial.len(), 9, "结构监听：2 下拉 + 2 工具栏 + 树(open+4) = 9，实际：{initial:?}");
            // 初始 input 监听：仅搜索框。
            assert_eq!(event_listeners(&m1, "input").len(), 1, "仅搜索框一个输入监听");

            // Toolbar「＋ 接口」（[1]）：点击展开下拉（backdrop + HTTP 接口 + 从 cURL 导入）。
            // 静态 id 随 LoadTemplate 内建，Mutation 不可见；按注册顺序取：
            // [0] ＋ 文件夹 [1] ＋ 接口 [2] 环境下拉 [3] 树行 open [4..7] 行内按钮 [8] 项目下拉。
            let add_ep = initial[1];
            dom.handle_event("click", mouse(), add_ep, true);
            let m_menu = dom.render_immediate_to_vec();
            let fresh_menu: Vec<ElementId> = event_listeners(&m_menu, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_menu.len(), 3, "下拉应为 backdrop + HTTP 接口 + 从 cURL 导入，实际：{fresh_menu:?}");

            // 点「HTTP 接口」→ 名称弹窗（backdrop + 弹窗体 + 取消 + 确定）。
            dom.handle_event("click", mouse(), fresh_menu[1], true);
            let m_dlg = dom.render_immediate_to_vec();
            let fresh_dlg: Vec<ElementId> = event_listeners(&m_dlg, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_dlg.len(), 4, "名称弹窗应为 backdrop+弹窗体+取消+确定，实际：{fresh_dlg:?}");
            // 点 backdrop 关闭。
            dom.handle_event("click", mouse(), fresh_dlg[0], true);
            let _m_dlg_closed = dom.render_immediate_to_vec();

            // 再点「＋ 接口」展开，点「从 cURL 导入」→ cURL 导入弹窗，backdrop 关闭。
            dom.handle_event("click", mouse(), add_ep, true);
            let m_menu2 = dom.render_immediate_to_vec();
            let fresh_menu2: Vec<ElementId> = event_listeners(&m_menu2, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_menu2.len(), 3, "再次展开应有 3 个新监听");
            dom.handle_event("click", mouse(), fresh_menu2[2], true);
            let m_curl = dom.render_immediate_to_vec();
            let fresh_curl: Vec<ElementId> = event_listeners(&m_curl, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_curl.len(), 4, "cURL 导入弹窗应为 backdrop+弹窗体+取消+解析并导入，实际：{fresh_curl:?}");
            dom.handle_event("click", mouse(), fresh_curl[0], true);
            let _m_curl_closed = dom.render_immediate_to_vec();

            // 底部环境下拉（[2]）：展开应新增 backdrop（无环境项）。
            let env_dd = initial[2];
            dom.handle_event("click", mouse(), env_dd, true);
            let m_env = dom.render_immediate_to_vec();
            let fresh_env: Vec<ElementId> = event_listeners(&m_env, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_env.len(), 1, "环境下拉应展开（仅 backdrop），实际：{fresh_env:?}");
            dom.handle_event("click", mouse(), fresh_env[0], true);
            let _m_env_closed = dom.render_immediate_to_vec();

            // 顶部项目下拉（[8]）：展开应新增 backdrop + 项目菜单项监听。
            let proj_dd = initial[8];
            dom.handle_event("click", mouse(), proj_dd, true);
            let m_menu = dom.render_immediate_to_vec();
            let fresh_menu: Vec<ElementId> = event_listeners(&m_menu, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_menu.len(), 2, "项目下拉应展开（backdrop + 1 项目项），实际：{fresh_menu:?}");
            dom.handle_event("click", mouse(), fresh_menu[0], true);
            let _m_menu_closed = dom.render_immediate_to_vec();

            // 接口行的「导入cURL」打开导入弹窗，backdrop 关闭。
            let row = listener_id_for(&m1, &format!("import-curl-{}", ep_id));
            assert_eq!(row.len(), 1, "接口行应有「导入cURL」按钮");
            dom.handle_event("click", mouse(), row[0], true);
            let m4 = dom.render_immediate_to_vec();
            let fresh_modal: Vec<ElementId> = event_listeners(&m4, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert_eq!(fresh_modal.len(), 4, "导入弹窗应为 backdrop+弹窗体+取消+解析并导入");
            dom.handle_event("click", mouse(), fresh_modal[0], true);
            let m5 = dom.render_immediate_to_vec();
            let fresh_close: Vec<ElementId> = event_listeners(&m5, "click")
                .iter()
                .filter(|id| !initial.contains(id))
                .copied()
                .collect();
            assert!(fresh_close.is_empty(), "backdrop 点击应关闭导入弹窗");
        });
        });
    }

    #[test]
    fn import_curl_from_row_creates_endpoint() {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let pool = fox_storage::db::memory_pool().await.unwrap();
            let project_id = Uuid::new_v4();
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
            let mut dom = VirtualDom::new_with_props(root, ());
                let m1 = dom.rebuild_to_vec();
                let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let ep_id = state.endpoints.read()[0].id;
                let clicks1 = event_listeners(&m1, "click");
                let inputs1 = event_listeners(&m1, "input");

                // 接口行「导入cURL」打开导入弹窗。
                let row = listener_id_for(&m1, &format!("import-curl-{}", ep_id));
                assert_eq!(row.len(), 1, "接口行应有「导入cURL」按钮");
                dom.handle_event("click", mouse(), row[0], true);
                let m2 = dom.render_immediate_to_vec();
                let clicks2 = event_listeners(&m2, "click");
                let modal_ids: Vec<ElementId> = clicks2
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(modal_ids.len(), 4, "应弹出导入弹窗");

                // 弹窗 textarea（新增的 input 监听）。
                let inputs2 = event_listeners(&m2, "input");
                let textarea_ids: Vec<ElementId> = inputs2
                    .iter()
                    .filter(|id| !inputs1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(textarea_ids.len(), 1, "应有一个 textarea");
                dom.handle_event(
                    "input",
                    form_input("curl -X POST -H 'Content-Type: application/json' -d '{\"a\":1}' https://api.example.com/users"),
                    textarea_ids[0],
                    true,
                );
                let _m3 = dom.render_immediate_to_vec();

                // 点「解析并导入」（新增监听中的最后一个）。
                dom.handle_event("click", mouse(), modal_ids[3], true);
                let m4 = dom.render_immediate_to_vec();

                // 弹窗应关闭。
                let clicks4 = event_listeners(&m4, "click");
                let fresh: Vec<ElementId> = clicks4
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert!(fresh.is_empty(), "导入后弹窗应关闭，残留监听：{fresh:?}");

                // 异步创建接口：泵几轮让 spawn 完成。
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let _ = dom.render_immediate_to_vec();
                }
                let eps = state.endpoints.read().clone();
                let toasts: Vec<String> = state
                    .toasts
                    .read()
                    .iter()
                    .map(|t| t.message.clone())
                    .collect();
                assert!(
                    eps.iter().any(|e| e.path == "https://api.example.com/users"
                        && e.method == HttpMethod::POST),
                    "应在项目根创建 POST 接口，实际：{:?}；toasts：{toasts:?}",
                    eps.iter().map(|e| (e.name.clone(), e.path.clone())).collect::<Vec<_>>()
                );
            });
    }
}