mod commands;
mod downloader;
mod models;
mod services;
mod utils;

use commands::download::DownloaderState;
use downloader::ytdlp::YtDlpDownloader;
use std::sync::Arc;

/// Initialize the application
pub fn run() {
    // Initialize logging
    let file_appender = tracing_appender::rolling::daily(
        utils::app_home::AppHome::config_dir(),
        "xdownload.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(non_blocking)
        .init();

    // Load saved language setting
    services::i18n::I18n::load_saved();

    // Initialize from environment variables first (lowest priority)
    services::proxy::ProxyConfig::init_from_environment();

    // Detect system proxy (higher priority than env vars, lower than saved)
    services::proxy::ProxyConfig::detect_system_proxy();

    // Apply saved proxy (highest priority — overrides env and system detection)
    services::config::ConfigManager::apply_saved_proxy();

    // Create the downloader
    let downloader = Arc::new(YtDlpDownloader::new());

    // Apply saved cookies
    let (browser, file) = services::config::ConfigManager::load_saved_cookies();
    if let Some(ref b) = browser {
        downloader.set_cookies_from_browser(b);
    } else if let Some(ref f) = file {
        downloader.set_cookies_file(f);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(DownloaderState {
            downloader: downloader.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::download::fetch_video_info,
            commands::download::start_download,
            commands::download::cancel_download,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::save_settings_to_path,
            commands::settings::get_download_dir,
            commands::settings::get_config_path,
            commands::settings::apply_saved_proxy,
            commands::settings::load_saved_cookies,
            commands::settings::apply_saved_cookies,
            commands::settings::save_and_apply_cookies,
            commands::settings::save_language,
            commands::settings::load_settings_from_path,
            commands::settings::apply_and_persist_settings,
            commands::settings::apply_default_config,
            commands::settings::save_as_default,
            commands::settings::get_disclaimer_accepted,
            commands::settings::accept_disclaimer,
            commands::proxy::test_proxy,
            commands::proxy::get_proxy_status,
            commands::proxy::set_proxy_mode,
            commands::cookies::validate_cookies,
            commands::cookies::scan_cookies,
            commands::bootstrap::check_ytdlp,
            commands::bootstrap::check_ffmpeg,
            commands::bootstrap::download_ytdlp,
            commands::bootstrap::download_ffmpeg,
            commands::bootstrap::ping_google,
            commands::bootstrap::cancel_bootstrap_download,
            commands::bootstrap::get_bin_dir,
            commands::bootstrap::get_root_dir,
            commands::bootstrap::open_root_dir,
            commands::bootstrap::get_config_dir,
            commands::bootstrap::open_config_dir,
            commands::bootstrap::quit_app,
            commands::bootstrap::get_uninstall_info,
            commands::bootstrap::uninstall_app,
            commands::bootstrap::open_uninstall_panel,
            commands::update::check_update,
            commands::update::check_ytdlp_update,
            commands::update::check_ffmpeg_update,
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running XDownload");
}

/// Return a global proxy state for the Tauri command layer.
pub fn proxy_global_state() -> std::sync::MutexGuard<'static, ()> {
    // A lightweight accessor — the actual state lives in services::proxy::ProxyConfig
    static DUMMY: std::sync::Mutex<()> = std::sync::Mutex::new(());
    DUMMY.lock().unwrap()
}
