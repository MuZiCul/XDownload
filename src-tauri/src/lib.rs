mod commands;
mod downloader;
mod models;
mod services;
mod tray;
mod utils;

use commands::download::DownloaderState;
use downloader::ytdlp::YtDlpDownloader;
use std::fs::{File, OpenOptions};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::format::Writer;

/// Writes each day's logs to `xdownload.log.YYYY-MM-DD` based on the LOCAL
/// date. (tracing-appender's built-in daily rotation uses UTC, which makes
/// the filename lag one day behind for UTC+ timezones.)
struct LocalDailyWriter {
    dir: PathBuf,
    /// Currently open file: (date string, file handle).
    current: Option<(String, File)>,
}

impl LocalDailyWriter {
    fn new(dir: PathBuf) -> Self {
        Self { dir, current: None }
    }

    fn ensure_file(&mut self) -> &mut File {
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let stale = self
            .current
            .as_ref()
            .map(|(d, _)| d != &date)
            .unwrap_or(true);
        if stale {
            let path = self.dir.join(format!("xdownload.log.{}", date));
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap_or_else(|_| {
                    File::create(&path).expect("failed to create log file")
                });
            self.current = Some((date, file));
        }
        &mut self.current.as_mut().expect("log file initialized").1
    }
}

impl IoWrite for LocalDailyWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.ensure_file().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some((_, f)) = &mut self.current {
            f.flush()
        } else {
            Ok(())
        }
    }
}

/// Timestamps log lines in the LOCAL timezone (previously UTC).
struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"))
    }
}

/// Initialize the application
pub fn run() {
    // Initialize logging: rotate by LOCAL date and stamp timestamps in the
    // local timezone (previously UTC, off by up to a day / 8 hours). Logs live
    // in logs/ (kept out of config/, and gitignored as a whole).
    let _ = utils::app_home::AppHome::ensure_logs_dir();
    let (non_blocking, _guard) = tracing_appender::non_blocking(LocalDailyWriter::new(
        utils::app_home::AppHome::logs_dir(),
    ));

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_timer(LocalTimer)
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

    // Wipe any partial files left in the download cache by a previous
    // interrupted session, so each session starts from a clean slate.
    YtDlpDownloader::cleanup_download_cache();

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
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .manage(DownloaderState {
            downloader: downloader.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::download::fetch_video_info,
            commands::download::check_video_downloaded,
            commands::download::start_download,
            commands::download::cancel_download,
            commands::download::is_downloading,
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
            commands::bootstrap::is_ffmpeg_bundled,
            commands::bootstrap::download_ytdlp,
            commands::bootstrap::download_ffmpeg,
            commands::bootstrap::ping_google,
            commands::bootstrap::cancel_bootstrap_download,
            commands::bootstrap::get_bin_dir,
            commands::bootstrap::get_root_dir,
            commands::bootstrap::open_root_dir,
            commands::bootstrap::get_config_dir,
            commands::bootstrap::open_config_dir,
            commands::bootstrap::open_download_dir,
            commands::bootstrap::open_download_path,
            commands::bootstrap::quit_app,
            commands::bootstrap::get_uninstall_info,
            commands::bootstrap::uninstall_app,
            commands::bootstrap::open_uninstall_panel,
            commands::update::check_update,
            commands::update::check_ytdlp_update,
            commands::update::check_ffmpeg_update,
            commands::update::download_update,
            commands::update::install_update,
            commands::history::list_download_history,
            commands::history::delete_download_history,
            commands::history::clear_download_history,
        ])
        .setup(|app| {
            // System tray icon + context menu.
            tray::init(app)?;

            // Clicking the window close button hides the app to the system tray
            // instead of quitting (the app keeps running in the background).
            // Real exits happen via the tray menu or the quit_app command.
            #[cfg(windows)]
            if let Some(win) = app.get_webview_window("main") {
                let tray_win = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = tray_win.hide();
                    }
                });
            }

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
