//! 临时复现测试：设置页环境编辑点击行为。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use fox_core::model::{Environment, Project};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use uuid::Uuid;

    use crate::pages::settings::SettingsPage;
    use crate::services::Services;
    use crate::state::AppState;

    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let project_id = PROJECT.with(|s| *s.borrow()).expect("项目已就绪");
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
        let env = Environment {
            id: ENV_ID.with(|s| *s.borrow()).unwrap_or_default(),
            project_id,
            name: "生产".into(),
            variables: HashMap::from([(
                "base_url".to_string(),
                "https://api.example.com".to_string(),
            )]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        state.environments.write().push(env);
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        use_context_provider(move || state);
        rsx! { SettingsPage {} }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
        static PROJECT: RefCell<Option<Uuid>> = const { RefCell::new(None) };
        static ENV_ID: RefCell<Option<Uuid>> = const { RefCell::new(None) };
    }

    fn mouse() -> Rc<dyn std::any::Any> {
        Rc::new(PlatformEventData::new(Box::new(
            dioxus_html::SerializedMouseData::default(),
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

    fn text_nodes(muts: &Mutations) -> Vec<String> {
        let mut out = Vec::new();
        for m in &muts.edits {
            if let Mutation::SetText { value, .. } = m {
                out.push(value.clone());
            }
        }
        out
    }

    #[test]
    fn edit_env_button_opens_editor() {
        dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let pool = fox_storage::db::memory_pool().await.unwrap();
            let project_id = Uuid::new_v4();
            let env_id = Uuid::new_v4();
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
            ENV_ID.with(|s| *s.borrow_mut() = Some(env_id));

            let mut dom = VirtualDom::new_with_props(root, ());
            let m1 = dom.rebuild_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");

            let clicks1 = event_listeners(&m1, "click");
            let inputs1 = event_listeners(&m1, "input");
            println!("初始 click 监听数：{}", clicks1.len());
            println!("初始 input 监听数：{}", inputs1.len());
            println!("初始文本：{:?}", text_nodes(&m1));

            // 逐个点击非 backdrop/dropdown 的 click 监听，观察是否能打开环境编辑器（出现保存环境按钮）。
            let mut found_editor = false;
            for (i, id) in clicks1.iter().enumerate() {
                let texts_before = text_nodes(&m1);
                dom.handle_event("click", mouse(), *id, true);
                let m = dom.render_immediate_to_vec();
                let clicks = event_listeners(&m, "click");
                let inputs = event_listeners(&m, "input");
                let fresh_clicks: Vec<ElementId> = clicks
                    .iter()
                    .filter(|c| !clicks1.contains(c))
                    .copied()
                    .collect();
                let fresh_inputs: Vec<ElementId> = inputs
                    .iter()
                    .filter(|c| !inputs1.contains(c))
                    .copied()
                    .collect();
                let texts_after: Vec<String> = text_nodes(&m)
                    .into_iter()
                    .filter(|t| !texts_before.contains(t))
                    .collect();
                println!(
                    "click[{i}] rgb{:?} -> 新增 click {fresh_clicks:?} 新增 input {fresh_inputs:?} 新增文本 {texts_after:?}",
                    id
                );
                if !fresh_clicks.is_empty() && fresh_inputs.len() >= 2 {
                    found_editor = true;
                    println!(">>> 编辑按钮疑似为 click[{i}]");
                    break;
                }
            }
            assert!(found_editor, "未找到能打开环境编辑器的按钮");
            let _ = state;
        });
    }
}
