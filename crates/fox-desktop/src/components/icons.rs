#![allow(non_snake_case)]
//! 内联 SVG 图标：统一 viewBox 0 0 24 24、fill none、stroke currentColor、stroke-width 2。

use dioxus::prelude::*;

fn svg_base(children: Element) -> Element {
    rsx! {
        svg {
            view_box: "0 0 24 24",
            width: "16",
            height: "16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            {children}
        }
    }
}

/// 搜索图标。
pub fn SearchIcon() -> Element {
    svg_base(rsx! {
        circle { cx: "11", cy: "11", r: "7" }
        path { d: "M21 21l-4.35-4.35" }
    })
}

/// 下箭头（下拉指示）。
pub fn CaretIcon() -> Element {
    svg_base(rsx! {
        path { d: "M6 9l6 6 6-6" }
    })
}

/// 加号。
pub fn PlusIcon() -> Element {
    svg_base(rsx! {
        path { d: "M12 5v14M5 12h14" }
    })
}

/// 文件夹。
pub fn FolderIcon() -> Element {
    svg_base(rsx! {
        path { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" }
    })
}

/// 关闭 / 失败 X 图标。
pub fn XIcon() -> Element {
    svg_base(rsx! {
        path { d: "M18 6L6 18" }
        path { d: "M6 6l12 12" }
    })
}

/// 设置（滑杆）。
pub fn SlidersIcon() -> Element {
    svg_base(rsx! {
        path { d: "M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6" }
    })
}
