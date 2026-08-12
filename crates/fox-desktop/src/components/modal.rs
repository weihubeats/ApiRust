//! 统一弹窗壳：入场 / 出场动画 + 遮罩点击关闭。

use std::time::Duration;

use dioxus::prelude::*;

/// 出场动画时长（与 styles.rs `.modal-exit` 动画时长一致）。
const EXIT_MS: u64 = 100;

/// 统一弹窗壳：挂载时由 CSS 播放入场动画；
/// 点击遮罩先播放出场动画（100ms），动画结束后回调 `on_close`（真正的关闭动作）。
/// 内容区点击不穿透。默认不可动画关闭的场景可传 `on_backdrop` 覆盖。
#[allow(non_snake_case)]
#[component]
pub fn RFModal(
    // 出场动画结束后调用（通常在外部把打开状态 Signal 置 false）。
    on_close: EventHandler<MouseEvent>,
    // 自定义遮罩点击行为（跳过出场动画），缺省走动画关闭。
    #[props(optional)] on_backdrop: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let mut closing = use_signal(|| false);
    let backdrop = match on_backdrop {
        Some(cb) => cb,
        None => {
            let on_close = on_close;
            EventHandler::new(move |e| {
                if !*closing.read() {
                    closing.set(true);
                    // 无 Tokio 运行时（如组件测试）时跳过动画、直接关闭。
                    if tokio::runtime::Handle::try_current().is_ok() {
                        spawn(async move {
                            tokio::time::sleep(Duration::from_millis(EXIT_MS)).await;
                            closing.set(false);
                            on_close.call(e);
                        });
                    } else {
                        closing.set(false);
                        on_close.call(e);
                    }
                }
            })
        }
    };
    let exiting = move || if *closing.read() { " modal-exit" } else { "" };
    rsx! {
        div {
            class: "modal-backdrop{exiting()}",
            onclick: move |e| backdrop.call(e),
            div {
                class: "modal{exiting()}",
                onclick: |e| e.stop_propagation(),
                {children}
            }
        }
    }
}
