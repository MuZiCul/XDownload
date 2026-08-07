//! System tray icon — keeps the app running in the background after the main
//! window is closed, provides a right-click context menu (show window / open
//! download dir / quit), and restores the main window on a left click.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

/// Create the tray icon, its context menu and event handlers.
pub fn init(app: &App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let downloads_i = MenuItem::with_id(app, "downloads", "打开下载目录", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &downloads_i, &quit_i])?;

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
            "quit" => {
                tracing::info!("tray: quitting via tray menu");
                // Same cleanup as the quit_app command — kill yt-dlp/ffmpeg
                // child processes so no download is left running.
                crate::utils::process::kill_all_children();
                app.exit(0);
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
