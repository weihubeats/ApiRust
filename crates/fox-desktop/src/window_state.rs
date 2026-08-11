//! 窗口状态持久化：几何信息（width/height/x/y）与最大化标记
//! 存 SQLite settings 表，关闭时写入、启动时恢复。

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use fox_storage::repository as repo;

/// settings 表中的键。
pub const WINDOW_STATE_KEY: &str = "window_state";

/// 窗口状态（逻辑坐标，与 DPI 无关）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub maximized: bool,
}

impl WindowState {
    /// 从 tao 窗口读取当前几何与最大化状态。
    pub fn from_window(w: &dioxus::desktop::tao::window::Window) -> Self {
        let scale = w.scale_factor();
        let size = w.inner_size();
        let pos = w.outer_position().unwrap_or_default();
        WindowState {
            width: size.width as f64 / scale,
            height: size.height as f64 / scale,
            x: pos.x as f64 / scale,
            y: pos.y as f64 / scale,
            maximized: w.is_maximized(),
        }
    }

    /// 启动时从 settings 表读取（无记录 / 损坏时返回 None）。
    pub fn load(pool: &SqlitePool) -> Option<WindowState> {
        let rt = tokio::runtime::Runtime::new().ok()?;
        let raw = rt
            .block_on(repo::get_setting(pool, WINDOW_STATE_KEY))
            .ok()
            .flatten()?;
        serde_json::from_str(&raw).ok()
    }

    /// 关闭时写入 settings 表（后台线程，避免阻塞事件循环）。
    pub fn save(pool: SqlitePool, state: WindowState) {
        let raw = serde_json::to_string(&state).unwrap_or_default();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(_) => return,
            };
            let _ = rt.block_on(repo::set_setting(&pool, WINDOW_STATE_KEY, &raw));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_roundtrip() {
        let s = WindowState {
            width: 1360.0,
            height: 900.0,
            x: 10.5,
            y: 20.25,
            maximized: true,
        };
        let raw = serde_json::to_string(&s).unwrap();
        let back: WindowState = serde_json::from_str(&raw).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn corrupt_json_returns_none_via_load_path() {
        // load 依赖数据库，此处验证解析失败路径（None 由调方回退默认窗口）。
        let bad: Result<WindowState, _> = serde_json::from_str("not json");
        assert!(bad.is_err());
    }
}
