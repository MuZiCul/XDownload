//! System tray icon — keeps the app running in the background after the main
//! window is closed, provides a right-click context menu (show window / open
//! download dir / quit), and restores the main window on a left click.

use std::sync::OnceLock;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Listener, Manager,
};

/// 托盘「开启/退出隐私模式」菜单项（克隆保存，供事件监听中更新文本）。
static PRIVACY_ITEM: OnceLock<MenuItem<tauri::Wry>> = OnceLock::new();

/// Create the tray icon, its context menu and event handlers.
pub fn init(app: &App) -> tauri::Result<()> {
    // 初始文本直接读持久化配置，应用启动时托盘菜单即显示正确状态。
    let privacy_text = if crate::services::config::ConfigManager::load_privacy_mode() {
        "退出隐私模式"
    } else {
        "开启隐私模式"
    };
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let downloads_i = MenuItem::with_id(app, "downloads", "打开下载目录", true, None::<&str>)?;
    let privacy_i = MenuItem::with_id(app, "privacy", privacy_text, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &downloads_i, &privacy_i, &quit_i])?;
    let _ = PRIVACY_ITEM.set(privacy_i.clone());

    // 前端隐私状态变更时（设置页/状态栏/托盘入口任一切换），同步托盘菜单文本。
    app.listen("privacy-mode-changed", |event| {
        let enabled = serde_json::from_str::<serde_json::Value>(event.payload())
            .ok()
            .and_then(|v| v.get("enabled").and_then(|e| e.as_bool()))
            .unwrap_or(false);
        let text = if enabled {
            "退出隐私模式"
        } else {
            "开启隐私模式"
        };
        if let Some(item) = PRIVACY_ITEM.get() {
            let _ = item.set_text(text);
        }
    });

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().expect("failed to load app icon"))
        .tooltip("XDownload")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "downloads" => {
                let _ = crate::commands::bootstrap::open_download_dir(app.clone());
            }
            "privacy" => {
                // 前端负责切换隐私状态（emit privacy-mode-changed 回来更新菜单文本）。
                tracing::info!("tray: privacy toggle requested");
                let _ = app.emit("toggle-privacy-mode", ());
            }
            "quit" => {
                // Let the frontend decide (exit-confirmation when tasks are
                // active). It will call quit_app(true/false) accordingly.
                tracing::info!("tray: quit requested via tray menu");
                let _ = app.emit(
                    "quit-requested",
                    serde_json::json!({ "source": "tray" }),
                );
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Show, unminimize and focus the main window.
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
