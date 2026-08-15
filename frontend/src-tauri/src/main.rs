//! RustFox Tauri 2 应用入口：装配 fox-tauri 插件（数据库初始化 + 全部 Command）。
//!
//! 原生菜单：应用菜单「About RustFox」（自定义，点击后向 WebView 广播
//! `rustfox://about` 事件以打开自定义关于弹窗）/「Hide RustFox」/「Quit RustFox」，
//! 另保留 Edit / View / Window 标准菜单（撤消/重做/剪切/复制/粘贴/全屏/最小化等）。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::Emitter;

const ABOUT_EVENT: &str = "rustfox://about";

fn build_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    let about = MenuItem::with_id(app, "about", "About RustFox", true, None::<&str>)?;
    let app_menu = Submenu::with_items(
        app,
        "RustFox",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some("Hide RustFox"))?,
            &PredefinedMenuItem::hide_others(app, Some("Hide Others"))?,
            &PredefinedMenuItem::show_all(app, Some("Show All"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Quit RustFox"))?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("Undo"))?,
            &PredefinedMenuItem::redo(app, Some("Redo"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some("Cut"))?,
            &PredefinedMenuItem::copy(app, Some("Copy"))?,
            &PredefinedMenuItem::paste(app, Some("Paste"))?,
            &PredefinedMenuItem::select_all(app, Some("Select All"))?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, Some("Toggle Full Screen"))?],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some("Minimize"))?,
            &PredefinedMenuItem::close_window(app, Some("Close Window"))?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])
}

fn main() {
    tauri::Builder::default()
        .plugin(fox_tauri::plugin::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            app.set_menu(build_menu(app)?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().0 == "about" {
                let _ = app.emit(ABOUT_EVENT, ());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running RustFox");
}