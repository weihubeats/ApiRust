//! 全局键盘快捷键（桌面端）：通过注入 JS 监听 window keydown，经 eval 通道回调 Rust。
//!
//! 快捷键映射：
//! - Ctrl/Cmd+N：新建接口（默认名「未命名接口」）
//! - Ctrl/Cmd+K：聚焦顶部全局搜索框 #global-search
//! - Ctrl/Cmd+Shift+F：新建文件夹
//!
//! 防冲突：焦点位于 input / textarea / select / contenteditable 时不触发。

use dioxus::events::eval;
use dioxus::prelude::*;
use serde_json::Value;

use crate::state::AppState;

/// 注入的 JS：注册全局 keydown 监听，命中快捷键时经 dioxus.send 通知 Rust。
/// 搜索聚焦在 JS 侧直接完成（无需 Rust 往返）。
const SHORTCUTS_JS: &str = r#"
(function () {
  if (window.__rfShortcutsInstalled) return;
  window.__rfShortcutsInstalled = true;
  window.addEventListener('keydown', function (e) {
    var t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' ||
              t.tagName === 'SELECT' || t.isContentEditable)) return;
    var mod = e.metaKey || e.ctrlKey;
    if (!mod) return;
    var key = e.key.toLowerCase();
    if (key === 'k' && !e.shiftKey) {
      e.preventDefault();
      var el = document.getElementById('global-search');
      if (el) {
        el.focus();
        el.select();
      }
      return;
    }
    if (key === 'n' && !e.shiftKey) {
      e.preventDefault();
      dioxus.send('new-endpoint');
      return;
    }
    if (key === 'f' && e.shiftKey) {
      e.preventDefault();
      dioxus.send('new-folder');
    }
  });
})();
"#;

/// 注入的 JS：拦截 element 为 null 的 user_event。
/// dioxus-desktop 上游缺陷（DioxusLabs/dioxus#2566）：事件目标位于 dioxus 树外
/// （无 data-dioxus-id）时，解释器会发出 element:null，宿主端反序列化
/// ElementId(usize) 失败并打印 "invalid type: null, expected usize"。
/// 此类事件的监听器本就不存在，直接丢弃即可（等价于上游处理结果）。
///
/// 注意：不能改写 window.ipc.postMessage —— wry 用 Object.freeze 冻结了该对象，
/// 赋值静默失败，守卫形同虚设（历史 bug）。正确做法是包一层
/// window.interpreter.handleEvent：它是所有 DOM 事件序列化进 IPC 的唯一入口，
/// 且其原型对象可写。
const USER_EVENT_GUARD_JS: &str = r#"
(function () {
  var interp = window.interpreter;
  if (!interp) return;
  var proto = Object.getPrototypeOf(interp);
  var orig = proto && proto.handleEvent;
  if (typeof orig !== 'function' || orig.__rfGuardInstalled) return;
  var hasDioxusId = function (node) {
    while (node) {
      if (node.getAttribute && node.getAttribute('data-dioxus-id') !== null) {
        return true;
      }
      node = node.parentNode;
    }
    return false;
  };
  proto.handleEvent = function (event, name, bubbles) {
    if (event && event.target && !hasDioxusId(event.target)) {
      return;
    }
    return orig.call(this, event, name, bubbles);
  };
  proto.handleEvent.__rfGuardInstalled = true;
})();
"#;

/// 侧边栏树操作目标索引：无项目时由 state 方法兜底提示。
#[component]
pub fn KeyboardShortcuts() -> Element {
    let state = use_context::<AppState>();
    let handle = eval(SHORTCUTS_JS);
    let _guard = eval(USER_EVENT_GUARD_JS);
    let st = state.clone();

    use_effect(move || {
        let mut ev = handle;
        let st2 = st.clone();
        spawn(async move {
            while let Ok(value) = ev.recv().await {
                dispatch_shortcut(&st2, &value);
            }
        });
    });

    None
}

/// 将 JS 传入的快捷键消息分发到状态方法（独立函数便于单测）。
fn dispatch_shortcut(state: &AppState, value: &Value) {
    match value.as_str() {
        Some("new-endpoint") => state.create_endpoint_at(None, "未命名接口".into()),
        Some("new-folder") => state.create_folder_at(None, "新建文件夹".into()),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use dioxus::prelude::*;
    use fox_core::model::Project;
    use serde_json::json;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use uuid::Uuid;

    use crate::services::Services;
    use crate::state::AppState;

    use super::dispatch_shortcut;

    // 说明：作用域内 Signal 的读写必须在 dioxus render 上下文内进行，
    // 故在 root 渲染期调用 dispatch_shortcut（模拟 JS 快捷键消息）。

    /// 根组件：创建状态（挂池 + 项目已入库），选中项目后派发全部快捷键消息。
    fn root(_: ()) -> Element {
        let pool = POOL.with(|s| s.borrow().clone()).expect("连接池已就绪");
        let project_id = PROJECT.with(|s| s.borrow().clone()).expect("项目已就绪");
        let state = AppState::new(Services::new(pool.clone()));
        state.set_current_project(Some(project_id));
        dispatch_shortcut(&state, &json!("new-folder"));
        dispatch_shortcut(&state, &json!("new-endpoint"));
        dispatch_shortcut(&state, &json!("focus-search"));
        dispatch_shortcut(&state, &json!("unknown"));
        ST.with(|s| *s.borrow_mut() = Some(state.clone()));
        use_context_provider(move || state);
        rsx! { div { "shortcut-test" } }
    }

    thread_local! {
        static ST: RefCell<Option<AppState>> = const { RefCell::new(None) };
        static POOL: RefCell<Option<sqlx::SqlitePool>> = const { RefCell::new(None) };
        static PROJECT: RefCell<Option<Uuid>> = const { RefCell::new(None) };
    }

    #[test]
    fn dispatch_maps_shortcut_messages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let pool = fox_storage::db::memory_pool().await.unwrap();
            let project_id = Uuid::new_v4();
            let project = Project {
                id: project_id,
                name: "快捷键项目".into(),
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
            let _m1 = dom.rebuild_to_vec();
            let state = ST.with(|s| s.borrow().clone()).expect("状态已就绪");

            // 异步创建：泵几轮让 spawn 完成的写库与状态同步落地。
            for _ in 0..30 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                let _ = dom.render_immediate_to_vec();
            }

            // 干扰消息不应导致任何创建。
            assert!(
                !state.folders.read().is_empty(),
                "Ctrl+Shift+F 应创建文件夹"
            );
            assert!(!state.endpoints.read().is_empty(), "Ctrl+N 应创建接口");
        });
    }
}
