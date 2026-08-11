//! 临时复现测试：从首页点击项目进入工作区后接口树是否立即出现。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use fox_core::model::{Endpoint, EndpointStatus, Folder, HttpMethod, Project, RequestSpec};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use uuid::Uuid;

    use crate::components::project_tree::SideBar;
    use crate::services::Services;
    use crate::state::{AppState, Page};

    /// 模拟首页：提供按钮，点击后 select_project 并把 current_page 切到 Workspace。
    /// 与真实 HomePage 一致：AppState 由 use_context_provider 在 root 提供（跨渲染稳定）。
    fn home_mock(_: ()) -> Element {
        let state = use_context::<AppState>();
        let st = state.clone();
        let pid = PROJECT.with(|s| *s.borrow()).expect("项目已就绪");
        rsx! {
            div { id: "home-mock",
                button { id: "select-project", onclick: move |_| st.select_project(pid) }
            }
        }
    }

    /// 模拟 App 根布局：与真实 app.rs 相同，use_context_provider 只创建一次 AppState。
    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let state = use_context_provider(|| AppState::new(Services::new(pool.clone())));
        ST.with(|s| {
            let mut slot = s.borrow_mut();
            if slot.is_none() {
                *slot = Some(state.clone());
            }
        });
        let has_project = state.current_project_id.read().is_some();
        let page = *state.current_page.read();
        rsx! {
            div { class: "app",
                div { class: "body",
                    if has_project {
                        SideBar {}
                    }
                    main { class: "main",
                        match page {
                            Page::Home => rsx! { home_mock {} },
                            Page::Workspace => rsx! { div { id: "workspace-mock", "工作区" } },
                            Page::Settings => rsx! { div { id: "settings-mock", "设置" } },
                        }
                    }
                }
            }
        }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
        static PROJECT: RefCell<Option<Uuid>> = const { RefCell::new(None) };
    }

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedMouseData::default(),
        )))
    }

    fn new_listener(m: &Mutation) -> Option<ElementId> {
        if let Mutation::NewEventListener { name: n, id, .. } = m {
            if n == "click" {
                return Some(*id);
            }
        }
        None
    }

    fn find_endpoint_texts(muts: &Mutations) -> Vec<String> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::SetText { value, .. } = m {
                if value.contains("健康检查") || value.contains("列表") {
                    out.push(value.clone());
                }
            }
            if let Mutation::HydrateText { value, .. } = m {
                if value.contains("健康检查") || value.contains("列表") {
                    out.push(value.clone());
                }
            }
            if let Mutation::CreateTextNode { value, .. } = m {
                if value.contains("健康检查") || value.contains("列表") {
                    out.push(value.clone());
                }
            }
        }
        out
    }

    #[test]
    fn first_project_selection_shows_endpoints() {
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
            let folder = Folder {
                id: Uuid::new_v4(),
                project_id,
                parent_id: None,
                name: "用户模块".into(),
                sort_order: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            fox_storage::repository::save_folder(&pool, &folder)
                .await
                .unwrap();
            let ep = Endpoint {
                id: Uuid::new_v4(),
                project_id,
                folder_id: None,
                name: "健康检查".into(),
                method: HttpMethod::GET,
                path: "/health".into(),
                description: String::new(),
                status: EndpointStatus::Developing,
                sort_order: 0,
                request: RequestSpec::default(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let ep2 = Endpoint {
                id: Uuid::new_v4(),
                project_id,
                folder_id: Some(folder.id),
                name: "用户列表".into(),
                method: HttpMethod::GET,
                path: "/api/users".into(),
                description: String::new(),
                status: EndpointStatus::Developing,
                sort_order: 0,
                request: RequestSpec::default(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            fox_storage::repository::save_endpoint(&pool, &ep)
                .await
                .unwrap();
            fox_storage::repository::save_endpoint(&pool, &ep2)
                .await
                .unwrap();
            POOL.with(|s| *s.borrow_mut() = Some(pool.clone()));
            PROJECT.with(|s| *s.borrow_mut() = Some(project_id));

            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let mut state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");
            state.projects.write().push(project);

            // 找到「select-project」按钮的 click 监听。
            let btn_id: Vec<ElementId> = m1.edits.iter().filter_map(new_listener).collect();
            assert_eq!(btn_id.len(), 1, "初始只有 select-project 按钮");

            // 模拟首次点击项目卡片。
            dom.handle_event("click", mouse(), btn_id[0], true);

            // 泵几轮让异步 refresh_project_data 完成（若任务被 HomePage 卸载 drop 则永远不出现）。
            let mut accumulated = Vec::new();
            for i in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                let m = dom.render_immediate_to_vec();
                let t = find_endpoint_texts(&m);
                if !t.is_empty() {
                    accumulated.extend(t);
                }
                if !accumulated.is_empty() {
                    println!(">>> 泵第 {i} 轮出现接口文本：{accumulated:?}");
                    break;
                }
            }
            let endpoints = state.endpoints.read().clone();
            println!(
                ">>> 最终 state.endpoints.len = {}，folders.len = {}",
                endpoints.len(),
                state.folders.read().len()
            );

            let m_final = dom.render_immediate_to_vec();
            let texts = find_endpoint_texts(&m_final);
            accumulated.extend(texts);
            println!(">>> 最终渲染中的接口文本：{accumulated:?}");
            assert_eq!(state.endpoints.read().len(), 2, "state 应加载到 2 个接口");
            assert!(
                !accumulated.is_empty(),
                "侧边栏应渲染出接口行，实际文本：{accumulated:?}"
            );
        });
    }
}
