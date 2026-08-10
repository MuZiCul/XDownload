// 大量功能函数经由 Tauri IPC 命令被前端调用，Rust 编译器无法感知这些
// 使用点，会误报 dead_code。统一抑制该警告（保留后续功能代码）。
#![allow(dead_code)]

mod commands;
mod downloader;
mod models;
mod services;
mod tray;
mod utils;

use commands::download::DownloaderState;
use downloader::queue::DownloadQueue;
use downloader::ytdlp::YtDlpDownloader;
use models::config::DownloadConfig;
use std::fs::{File, OpenOptions};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
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

    // Clean the download cache on the first launch of 4/14/24 (date contains
    // "4"). On other days the cache (.part files) is kept so interrupted
    // downloads can be resumed across sessions.
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // 必须放在 deep-link 之前：Windows 深链通过「协议拉起新实例」实现，
        // 需 single-instance 把深链 URL 参数转发给已运行的主实例。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").map(|w| {
                if w.is_minimized().unwrap_or(false) {
                    let _ = w.unminimize();
                }
                let _ = w.show();
                let _ = w.set_focus();
            });
        }))
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            commands::download::fetch_video_info,
            commands::download::check_video_downloaded,
            commands::download::enqueue_download,
            commands::download::cancel_queue_task,
            commands::download::cancel_all_tasks,
            commands::download::has_active_tasks,
            commands::download::clear_download_queue,
            commands::download::start_queue,
            commands::download::pause_queue,
            commands::download::resume_queue,
            commands::download::pause_queue_task,
            commands::download::resume_queue_task,
            commands::download::pause_all_tasks,
            commands::download::resume_all_tasks,
            commands::download::queue_status,
            commands::download::update_task_info,
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
            commands::bootstrap::open_logs_dir,
            commands::bootstrap::open_download_dir,
            commands::bootstrap::open_download_path,
            commands::bootstrap::open_file_path,
            commands::settings::get_privacy_mode,
            commands::settings::set_privacy_mode,
            commands::settings::get_version,
            commands::bootstrap::quit_app,
            commands::bootstrap::get_uninstall_info,
            commands::bootstrap::uninstall_app,
            commands::bootstrap::open_uninstall_panel,
            commands::update::check_ytdlp_update,
            commands::update::check_ffmpeg_update,
            commands::history::list_download_history,
            commands::history::delete_download_history,
            commands::history::clear_download_history,
        ])
        .setup(move |app| {
            // 窗口标题带上版本号（版本号唯一数据源 = Cargo.toml）。
            if let Some(win) = app.get_webview_window("main") {
                let title = format!("XDownload v{}", app.package_info().version);
                let _ = win.set_title(&title);
            }

            // System tray icon + context menu.
            tray::init(app)?;

            // The multi-task queue needs the app handle for events — created
            // here and managed so commands can access it.
            let queue = Arc::new(DownloadQueue::new(app.handle().clone(), downloader.clone()));
            app.manage(DownloaderState {
                downloader: downloader.clone(),
                queue: queue.clone(),
            });
            // Restore unfinished multi-task downloads when the persist setting
            // is enabled (re-enqueues and starts draining).
            queue.restore_if_enabled();

            // 浏览器扩展深链：xdownload://add?url=<encoded status url>
            // 已运行时由 single-instance 转发 URL 回调；未运行时协议拉起应用，
            // URL 作为命令行参数传入，需用 get_current() 读取（on_open_url 的
            // 事件在插件 setup 阶段 emit，早于本回调注册，启动 URL 会丢失）。
            {
                // 注册 xdownload:// 协议（写 HKCU\Software\Classes\xdownload，
                // 指向当前 exe）。插件不会自动注册，必须显式调用。
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("deep-link: register_all failed: {}", e);
                }

                // 运行中收到的深链 URL（single-instance 转发 / 窗口消息）。
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        handle_deep_link(&handle, url.as_str());
                    }
                });

                // 启动时（协议拉起）的深链 URL。
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    for url in urls {
                        handle_deep_link(app.handle(), url.as_str());
                    }
                }
            }

            // Clicking the window close button hides the app to the tray. When
            // tasks are active, the frontend is asked to confirm (save progress
            // dialog); otherwise the window hides directly — decided here in the
            // backend so the behavior never depends on frontend event delivery.
            #[cfg(windows)]
            if let Some(win) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        // Only capture the Send-able AppHandle in the callback.
                        let has_active = handle
                            .try_state::<crate::commands::download::DownloaderState>()
                            .map(|s| s.queue.has_active())
                            .unwrap_or(false);
                        if has_active {
                            let _ = handle.emit(
                                "quit-requested",
                                serde_json::json!({ "source": "close" }),
                            );
                        } else {
                            // Hide via the app handle (WebviewWindow is !Send,
                            // so it cannot be moved into this callback).
                            if let Some(main) = handle.get_webview_window("main") {
                                let _ = main.hide();
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running XDownload");
}

/// Validate a deep-link URL, then enqueue the target as a download task.
fn handle_deep_link(app: &tauri::AppHandle, raw: &str) {
    let Some(target) = parse_deep_link_url(raw) else {
        tracing::warn!("deep-link: invalid url {}", raw);
        return;
    };
    if !commands::download::is_supported_url(&target) {
        tracing::warn!("deep-link: rejected non-X url {}", target);
        return;
    }
    tracing::info!("deep-link: received {}", target);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(state) = handle.try_state::<commands::download::DownloaderState>() {
            // 绝对路径兜底：协议拉起时进程 cwd 可能是 system32，相对路径
            // "downloads" 会解析到错误位置，必须用安装目录下的 downloads。
            let output_dir = services::config::ConfigManager::load_download_dir()
                .unwrap_or_else(|| {
                    utils::app_home::AppHome::downloads_dir()
                        .to_string_lossy()
                        .to_string()
                });
            // 从 status URL 提取 id 填入 video_id：历史记录以 video_id 为键，
            // 缺失会导致下载完成后不写下载历史。
            let video_id = commands::download::extract_status_id(&target);

            // 深链来源单独处理：先 fetch 视频信息再入队（与普通 UI 流程一致），
            // 入队时就把 info 带上，保证任务卡片与下载历史的信息完整。
            // fetch 失败仍入队（不带 info，下载照常，yt-dlp 下载时会自行解析）。
            let fetch_result = state.downloader.fetch_video_info(&target).await;
            let (title, info) = match fetch_result {
                Ok(info) => {
                    tracing::info!("deep-link: info fetched for {}", target);
                    (info.title.clone(), serde_json::to_value(&info).ok())
                }
                Err(e) => {
                    tracing::warn!("deep-link: fetch info failed: {}", e);
                    (None, None)
                }
            };

            let config = DownloadConfig {
                url: target.clone(),
                video_id,
                title: title.clone(),
                thumbnail: None,
                uploader: None,
                duration: 0,
                view_count: 0,
                like_count: 0,
                format_id: "bestvideo+bestaudio/best".to_string(),
                output_dir,
                output_template: "%(title)s.%(ext)s".to_string(),
                extract_audio: false,
                embed_subtitles: false,
                embed_thumbnail: false,
                write_thumbnail: false,
                proxy: None,
                socket_timeout: 30,
                cookies_file: None,
                cookies_from_browser: None,
                max_height: 0,
                download_archive: None,
                playlist_items: None,
            };
            match state.queue.enqueue(config, title, true, info) {
                Ok(id) => {
                    tracing::info!("deep-link: enqueued task {} for {}", id, target);
                    // 专用事件：告知前端「已从浏览器获得下载任务」（普通入队的
                    // download-queued 不区分来源，这里单独发一个事件，前端据此
                    // 弹出 toast 并跳转到任务页）。
                    let _ = handle.emit(
                        "deep-link-queued",
                        serde_json::json!({ "task_id": id }),
                    );
                }
                Err(e) => tracing::warn!("deep-link: enqueue failed: {}", e),
            }
        }
    });
}

/// Parse a deep-link URL of the form `xdownload://add?url=<percent-encoded>`.
/// Returns the decoded target URL, or `None` for any other host/action.
/// Note: some browsers/OS normalize the empty path to a slash (`xdownload://add/`),
/// so the host is trimmed of a trailing slash before matching.
fn parse_deep_link_url(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("xdownload://")?;
    let (host, query) = rest.split_once('?')?;
    let host = host.trim_end_matches('/');
    if host != "add" {
        return None;
    }
    let mut target = None;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next()?;
        let v = it.next()?;
        if k == "url" {
            target = Some(percent_decode_str(v));
        }
    }
    target
}

/// Minimal percent-decode (%XX → byte). The browser extension encodes the
/// status URL with `encodeURIComponent`, so `%` sequences are expected.
fn percent_decode_str(s: &str) -> String {
    fn hex_val(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex_val(b[i + 1]), hex_val(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deep_link_standard() {
        let target = parse_deep_link_url(
            "xdownload://add?url=https%3A%2F%2Fx.com%2Fuser%2Fstatus%2F123%2Fvideo%2F1",
        );
        assert_eq!(
            target.as_deref(),
            Some("https://x.com/user/status/123/video/1")
        );
    }

    #[test]
    fn test_parse_deep_link_trailing_slash_host() {
        // 部分浏览器把空路径规范化为 add/。
        let target = parse_deep_link_url("xdownload://add/?url=https%3A%2F%2Ftwitter.com%2Fa");
        assert_eq!(target.as_deref(), Some("https://twitter.com/a"));
    }

    #[test]
    fn test_parse_deep_link_rejects_bad_host() {
        assert_eq!(parse_deep_link_url("xdownload://other?url=https://x.com/a"), None);
        assert_eq!(parse_deep_link_url("xdownload://add"), None);
        assert_eq!(parse_deep_link_url("https://x.com/a"), None);
        assert_eq!(parse_deep_link_url(""), None);
    }

    #[test]
    fn test_parse_deep_link_missing_url_param() {
        assert_eq!(parse_deep_link_url("xdownload://add?foo=bar"), None);
    }

    #[test]
    fn test_percent_decode_str() {
        assert_eq!(percent_decode_str("abc"), "abc");
        assert_eq!(percent_decode_str("a%20b"), "a b");
        assert_eq!(percent_decode_str("%3A%2F%2F"), "://");
        // 中文 UTF-8 三字节。
        assert_eq!(percent_decode_str("%E6%88%91"), "我");
        // 大小写十六进制。
        assert_eq!(percent_decode_str("%2f%2F"), "//");
        // 无效的 % 序列原样保留。
        assert_eq!(percent_decode_str("%"), "%");
        assert_eq!(percent_decode_str("%GG"), "%GG");
    }
}
