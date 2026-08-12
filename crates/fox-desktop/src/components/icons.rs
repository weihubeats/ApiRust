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

/// 粘贴 / 导入（剪贴板）。
pub fn ImportIcon() -> Element {
    svg_base(rsx! {
        rect { x: "8", y: "2", width: "8", height: "4", rx: "1", ry: "1" }
        path { d: "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" }
    })
}

/// 发送（纸飞机）。
pub fn SendIcon() -> Element {
    svg_base(rsx! {
        path { d: "m22 2-7 20-4-9-9-4Z" }
        path { d: "M22 2 11 13" }
    })
}

/// 历史（时钟）。
pub fn ClockIcon() -> Element {
    svg_base(rsx! {
        circle { cx: "12", cy: "12", r: "10" }
        path { d: "M12 6v6l4 2" }
    })
}

/// 保存（磁盘）。
pub fn SaveIcon() -> Element {
    svg_base(rsx! {
        path { d: "M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" }
        path { d: "M17 21v-8H7v8M7 3v5h8" }
    })
}

/// 生成代码（尖括号）。
pub fn CodeIcon() -> Element {
    svg_base(rsx! {
        path { d: "m16 18 6-6-6-6" }
        path { d: "m8 6-6 6 6 6" }
    })
}

/// 反馈（气泡）。
pub fn BugIcon() -> Element {
    svg_base(rsx! {
        path { d: "M8 2l1.88 1.88M14.12 3.88L16 2M9 7.13v-1a3 3 0 0 1 6 0v1" }
        path { d: "M12 20c-3.3 0-6-2.7-6-6v-3a6 6 0 0 1 12 0v3c0 3.3-2.7 6-6 6z" }
        path { d: "M12 20v2M6 13H2M22 13h-4M6 17l-4 1M22 18l-4-1" }
    })
}

/// Logo：赛博朋克狐狸脸（圆脸 + 尖耳 + 耳机弧 + 发光节点）。
pub fn FoxFaceIcon() -> Element {
    svg_base(rsx! {
        path { d: "M7.5 3.5 9.8 7.2M16.5 3.5 14.2 7.2" }
        path { d: "M9.8 7.2h4.4c1.6.3 2.9 1.7 2.9 3.3 0 .8-.3 1.6-.8 2.2.5.6.8 1.4.8 2.2 0 1.9-1.5 3.4-3.4 3.4h-5.4c-1.9 0-3.4-1.5-3.4-3.4 0-.8.3-1.6.8-2.2-.5-.6-.8-1.4-.8-2.2 0-1.6 1.3-3 2.9-3.3z" }
        circle { cx: "10", cy: "11.2", r: "1" }
        circle { cx: "14", cy: "11.2", r: "1" }
        path { d: "M12 15.6c.6 0 1-.4 1-1h-2c0 .6.4 1 1 1z" }
    })
}

/// 重命名（铅笔）。
pub fn PencilIcon() -> Element {
    svg_base(rsx! {
        path { d: "M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" }
    })
}

/// 复制。
pub fn CopyIcon() -> Element {
    svg_base(rsx! {
        rect { x: "9", y: "9", width: "13", height: "13", rx: "2", ry: "2" }
        path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
    })
}

/// 删除（垃圾桶）。
pub fn TrashIcon() -> Element {
    svg_base(rsx! {
        path { d: "M3 6h18" }
        path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
        path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
        path { d: "M10 11v6M14 11v6" }
    })
}

/// 侧边栏空状态插画（大尺寸文件夹 + 加号）。
pub fn TreeFolderIcon() -> Element {
    rsx! {
        svg {
            view_box: "0 0 24 24",
            width: "22",
            height: "22",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.6",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" }
            path { d: "M12 11v6M9 14h6" }
        }
    }
}
