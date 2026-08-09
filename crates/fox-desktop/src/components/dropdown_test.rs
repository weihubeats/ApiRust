//! Dropdown 组件行为测试（无头渲染 + 事件模拟）。

#[cfg(test)]
mod tests {
    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
    use dioxus::prelude::*;
    use std::rc::Rc;

    use crate::components::dropdown::Dropdown;

    fn probe() -> Element {
        let mut received = use_signal(String::new);
        rsx! {
            Dropdown {
                options: vec![
                    ("a".into(), "项目A".into()),
                    ("b".into(), "项目B".into()),
                ],
                selected: String::new(),
                placeholder: "未选择",
                on_select: move |v: String| received.set(v),
            }
            p { id: "received", "{received}" }
        }
    }

    /// 按顺序收集带 click 监听的元素 id。
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

    fn has_text(muts: &Mutations, text: &str) -> bool {
        for m in &muts.edits {
            if let Mutation::SetText { value, .. } = m {
                if value == text {
                    return true;
                }
            }
        }
        false
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
    fn click_trigger_opens_menu_and_select_fires() {
        with_converter(|| {
            let mut dom = VirtualDom::new(probe);
            let m1 = dom.rebuild_to_vec();

            let trigger = click_listeners(&m1);
            assert_eq!(trigger.len(), 1, "初始只有一个 click 监听（trigger）");
            assert!(!has_text(&m1, "项目A"));

            // 点击 trigger → 菜单应打开（新增 backdrop + 两个选项的 click 监听）
            dom.handle_event("click", mouse(), trigger[0], true);
            let m2 = dom.render_immediate_to_vec();
            let after_open = click_listeners(&m2);
            assert_eq!(after_open.len(), 3, "打开后应有 backdrop + 2 个选项的监听");

            // 点击第一个选项 → 菜单关闭，on_select 传出 "a"
            dom.handle_event("click", mouse(), after_open[1], true);
            let m3 = dom.render_immediate_to_vec();
            let after_close = click_listeners(&m3);
            assert_eq!(after_close.len(), 0, "选中后应无新增 click 监听");
            assert!(
                has_text(&m3, "a"),
                "on_select 应把选中值传给父组件并渲染出来"
            );
        });
    }

    #[test]
    fn backdrop_click_closes_menu() {
        with_converter(|| {
            let mut dom = VirtualDom::new(probe);
            let m1 = dom.rebuild_to_vec();
            let trigger = click_listeners(&m1)[0];

            dom.handle_event("click", mouse(), trigger, true);
            let m2 = dom.render_immediate_to_vec();
            let after_open = click_listeners(&m2);
            assert_eq!(after_open.len(), 3);

            // 点击 backdrop（第一个新监听）→ 菜单关闭
            dom.handle_event("click", mouse(), after_open[0], true);
            let m3 = dom.render_immediate_to_vec();
            assert_eq!(click_listeners(&m3).len(), 0);
            assert!(!has_text(&m3, "a"), "关闭菜单不应触发选择");
        });
    }
}
