//! RustFox Tauri 2 应用入口：装配 fox-tauri 插件（数据库初始化 + 全部 Command）。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(fox_tauri::plugin::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running RustFox");
}