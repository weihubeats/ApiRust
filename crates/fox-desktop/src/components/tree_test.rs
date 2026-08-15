//! 侧边栏行为测试（无头渲染 + 事件模拟）：
//! - 布局：Header 项目选择器 / Toolbar 双按钮 / Search / Tree；
//! - 「＋ 接口」弹名称弹窗并按名称建接口；接口行「导入cURL」弹窗导入并创建接口。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
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
        let project_id = PROJECT.with(|s| *s.borrow()).expect("项目已就绪");
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
        with_pool(|| {
            with_converter(|| {
                let mut dom = VirtualDom::new_with_props(root, ());
                let m1 = dom.rebuild_to_vec();

                // 初始 click 监听：2 下拉 + 2 工具栏 + 接口行(open + 重命名/复制/删除) = 7。
                let initial = event_listeners(&m1, "click");
                assert_eq!(
                    initial.len(),
                    7,
                    "结构监听：2 下拉 + 2 工具栏 + 接口行 4 = 7，实际：{initial:?}"
                );
                // 初始 input 监听：仅搜索框。
                assert_eq!(
                    event_listeners(&m1, "input").len(),
                    1,
                    "仅搜索框一个输入监听"
                );

                // Toolbar「＋ 接口」（[1]）：点击展开下拉（backdrop + HTTP 接口 + 从 cURL 导入）。
                // 静态 id 随 LoadTemplate 内建，Mutation 不可见；按注册顺序取：
                // [0] ＋ 文件夹 [1] ＋ 接口 [2] 接口行 open [3] 重命名 [4] 复制 [5] 删除 [6] 项目下拉。
                let add_ep = initial[1];
                dom.handle_event("click", mouse(), add_ep, true);
                let m_menu = dom.render_immediate_to_vec();
                let fresh_menu: Vec<ElementId> = event_listeners(&m_menu, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert_eq!(
                    fresh_menu.len(),
                    3,
                    "下拉应为 backdrop + HTTP 接口 + 从 cURL 导入，实际：{fresh_menu:?}"
                );

                // 点「HTTP 接口」→ 名称弹窗（backdrop + 弹窗体 + 取消 + 确定）。
                dom.handle_event("click", mouse(), fresh_menu[1], true);
                let m_dlg = dom.render_immediate_to_vec();
                let fresh_dlg: Vec<ElementId> = event_listeners(&m_dlg, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert_eq!(
                    fresh_dlg.len(),
                    4,
                    "名称弹窗应为 backdrop+弹窗体+取消+确定，实际：{fresh_dlg:?}"
                );
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
                assert_eq!(
                    fresh_curl.len(),
                    4,
                    "cURL 导入弹窗应为 backdrop+弹窗体+取消+解析并导入，实际：{fresh_curl:?}"
                );
                dom.handle_event("click", mouse(), fresh_curl[0], true);
                let _m_curl_closed = dom.render_immediate_to_vec();

                // 顶部项目下拉（[6]）：展开应新增 backdrop + 项目菜单项监听。
                let proj_dd = initial[6];
                dom.handle_event("click", mouse(), proj_dd, true);
                let m_menu = dom.render_immediate_to_vec();
                let fresh_menu: Vec<ElementId> = event_listeners(&m_menu, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert_eq!(
                    fresh_menu.len(),
                    2,
                    "项目下拉应展开（backdrop + 1 项目项），实际：{fresh_menu:?}"
                );
                dom.handle_event("click", mouse(), fresh_menu[0], true);
                let _m_menu_closed = dom.render_immediate_to_vec();

                // 接口行重命名（[3]）：点击 → 名称弹窗（backdrop + 弹窗体 + 取消 + 确定）。
                let row_rename = initial[3];
                dom.handle_event("click", mouse(), row_rename, true);
                let m_rename = dom.render_immediate_to_vec();
                let fresh_rename: Vec<ElementId> = event_listeners(&m_rename, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert_eq!(
                    fresh_rename.len(),
                    4,
                    "重命名弹窗应为 backdrop+弹窗体+取消+确定，实际：{fresh_rename:?}"
                );
                // 点 backdrop 关闭。
                dom.handle_event("click", mouse(), fresh_rename[0], true);
                let _m_rename_closed = dom.render_immediate_to_vec();
                // 接口行删除（[5]）：点击 → 确认删除弹窗，backdrop 关闭后无残留。
                let row_delete = initial[5];
                dom.handle_event("click", mouse(), row_delete, true);
                let m_confirm = dom.render_immediate_to_vec();
                let fresh_confirm: Vec<ElementId> = event_listeners(&m_confirm, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert_eq!(
                    fresh_confirm.len(),
                    4,
                    "删除确认弹窗应为 backdrop+弹窗体+取消+确定，实际：{fresh_confirm:?}"
                );
                dom.handle_event("click", mouse(), fresh_confirm[0], true);
                let m_close = dom.render_immediate_to_vec();
                let fresh_close: Vec<ElementId> = event_listeners(&m_close, "click")
                    .iter()
                    .filter(|id| !initial.contains(id))
                    .copied()
                    .collect();
                assert!(
                    fresh_close.is_empty(),
                    "backdrop 点击应关闭删除确认弹窗，残留：{fresh_close:?}"
                );
            });
        });
    }

    #[test]
    fn import_curl_from_toolbar_creates_endpoint() {
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
                let mut state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let clicks1 = event_listeners(&m1, "click");
                let inputs1 = event_listeners(&m1, "input");

                // Toolbar「＋ 接口」（[1]）展开下拉 → 点「从 cURL 导入」（新增监听 [2]）打开导入弹窗。
                let add_ep = clicks1[1];
                dom.handle_event("click", mouse(), add_ep, true);
                let m_menu = dom.render_immediate_to_vec();
                let fresh_menu: Vec<ElementId> = event_listeners(&m_menu, "click")
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(fresh_menu.len(), 3, "下拉应为 backdrop + HTTP 接口 + 从 cURL 导入，实际：{fresh_menu:?}");
                dom.handle_event("click", mouse(), fresh_menu[2], true);
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
                    eps.iter().all(|e| e.path != "https://api.example.com/users"),
                    "导入不应直接落库，实际：{:?}",
                    eps.iter().map(|e| (e.name.clone(), e.path.clone())).collect::<Vec<_>>()
                );
                let draft = state.unsaved_draft.read().clone();
                assert!(
                    draft.as_ref().is_some_and(|e| e.path == "https://api.example.com/users"
                        && e.method == HttpMethod::POST),
                    "导入应生成为未保存草稿，实际：{draft:?}；toasts：{toasts:?}"
                );
                assert!(
                    toasts.iter().any(|t| t.contains("草稿")),
                    "应提示已导入为草稿：{toasts:?}"
                );

                // Ctrl+S 保存未落库草稿 → 应进入命名确认（不直接落库）。
                let draft_ep = draft.unwrap();
                state.active_draft.set(Some(draft_ep.clone()));
                state.save_active_endpoint();
                let pending = state.pending_save.read().clone();
                assert!(
                    pending.as_ref().is_some_and(|e| e.id == draft_ep.id),
                    "未落库接口保存应弹命名确认框，实际：{pending:?}"
                );
                // 命名确认：关闭弹窗并触发保存（异步落库由持久化任务完成，此处验证状态流转）。
                state.confirm_save_name("导入接口 POST users".into());
                assert!(
                    state.pending_save.read().is_none(),
                    "确认后应关闭命名弹窗"
                );
            });
    }
}
