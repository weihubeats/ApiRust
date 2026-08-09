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

    fn form_input(value: &str) -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedFormData::new(value.to_string(), HashMap::new(), None),
        )))
    }

    /// 特定事件名（click / input）的监听元素 id（按挂载顺序）。
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
    fn import_curl_opens_and_closes_modal() {
        with_pool(|| { with_converter(|| {
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
                let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
                let clicks1 = event_listeners(&m1, "click");
                let inputs1 = event_listeners(&m1, "input");

                // 顶栏「导入 cURL」是第三个 click 监听（＋ 文件夹、＋ 接口、导入 cURL）。
                let toolbar_btn = clicks1[2];
                dom.handle_event("click", mouse(), toolbar_btn, true);
                let m2 = dom.render_immediate_to_vec();
                let clicks2 = event_listeners(&m2, "click");
                let modal_ids: Vec<ElementId> = clicks2
                    .iter()
                    .filter(|id| !clicks1.contains(id))
                    .copied()
                    .collect();
                assert_eq!(modal_ids.len(), 4, "顶栏导入应弹出弹窗");

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