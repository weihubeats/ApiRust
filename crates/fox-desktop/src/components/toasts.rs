//! 全局 Toast 容器。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::state::AppState;

/// Toast 存活时长。
const TOAST_KEEP: Duration = Duration::from_secs(4);
/// 消失前提前标记淡出的提前量（轮询 500ms，取 600ms 保证至少一次 tick 落在淡出窗口）。
const TOAST_FADE_LEAD: Duration = Duration::from_millis(600);

#[component]
pub fn Toasts() -> Element {
    let state = use_context::<AppState>();
    let mut toasts = state.toasts;
    let mut fading = use_signal(HashMap::<u64, Instant>::new);
    let items = toasts.read().clone();

    // 周期性清理过期 Toast（覆盖所有直接写入的 Toast，统一 4 秒自动消失）。
    // Dioxus 0.5 的 use_effect 每次重渲染都重跑：若直接 spawn，每次 toast
    // 推送/淡出/过期都会新增一个永不退出的轮询任务，造成内存持续增长。
    // 用标志保证整棵生命周期内只注册一次轮询任务。
    let started = use_hook(|| std::cell::Cell::new(false));
    use_effect(move || {
        if started.get() {
            return;
        }
        started.set(true);
        spawn(async move {
            let mut born: HashMap<u64, Instant> = HashMap::new();
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let now = Instant::now();
                let ids: Vec<u64> = toasts.read().iter().map(|t| t.id).collect();
                for id in &ids {
                    born.entry(*id).or_insert_with(Instant::now);
                }
                // 到期前先标记淡出（加 .fading 类），到期的直接移除。
                let fade_ids: Vec<u64> = born
                    .iter()
                    .filter(|(id, t)| {
                        ids.contains(id)
                            && now.duration_since(**t) >= TOAST_KEEP - TOAST_FADE_LEAD
                            && !fading.read().contains_key(id)
                    })
                    .map(|(id, _)| *id)
                    .collect();
                if !fade_ids.is_empty() {
                    fading
                        .write()
                        .extend(fade_ids.into_iter().map(|id| (id, now)));
                }
                let expired: Vec<u64> = born
                    .iter()
                    .filter(|(id, t)| now.duration_since(**t) >= TOAST_KEEP && ids.contains(id))
                    .map(|(id, _)| *id)
                    .collect();
                if !expired.is_empty() {
                    toasts.write().retain(|t| !expired.contains(&t.id));
                    fading.write().retain(|id, _| !expired.contains(id));
                }
                born.retain(|id, _| ids.contains(id));
            }
        });
    });

    rsx! {
        div { class: "rf-toast-wrap",
            for t in items {
                div {
                    class: format!(
                        "rf-toast {}{}",
                        t.kind.css_class(),
                        if fading.read().contains_key(&t.id) { " fading" } else { "" }
                    ),
                    "{t.message}"
                }
            }
        }
    }
}
