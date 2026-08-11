//! 全局 Toast 容器。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::state::AppState;

/// Toast 存活时长。
const TOAST_KEEP: Duration = Duration::from_secs(4);

#[component]
pub fn Toasts() -> Element {
    let state = use_context::<AppState>();
    let mut toasts = state.toasts;
    let items = toasts.read().clone();

    // 周期性清理过期 Toast（覆盖所有直接写入的 Toast，统一 4 秒自动消失）。
    use_effect(move || {
        spawn(async move {
            let mut born: HashMap<u64, Instant> = HashMap::new();
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let now = Instant::now();
                let ids: Vec<u64> = toasts.read().iter().map(|t| t.id).collect();
                for id in &ids {
                    born.entry(*id).or_insert_with(Instant::now);
                }
                let expired: Vec<u64> = born
                    .iter()
                    .filter(|(id, t)| now.duration_since(**t) >= TOAST_KEEP && ids.contains(id))
                    .map(|(id, _)| *id)
                    .collect();
                if !expired.is_empty() {
                    toasts.write().retain(|t| !expired.contains(&t.id));
                }
                born.retain(|id, _| ids.contains(id));
            }
        });
    });

    rsx! {
        div { class: "rf-toast-wrap",
            for t in items {
                div { class: "rf-toast {t.kind.css_class()}", "{t.message}" }
            }
        }
    }
}
