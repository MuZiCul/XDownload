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
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
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

/// 日志文件保留天数：早于该天数的 `xdownload.log.YYYY-MM-DD` 会在每日首次
/// 启动时被删除。
const LOG_RETENTION_DAYS: i64 = 14;
/// 日志清理 marker 文件名（记录上次执行清理的日期，保证每天只清理一次）。
const LOG_CLEANUP_MARKER: &str = "log_cleanup_date";

/// 每天第一次启动时清理超过 [`LOG_RETENTION_DAYS`] 天的日志文件。
///
/// 与 `cleanup_download_cache` 相同模式：用 marker 文件记住上次清理日期，
/// 同一天重复启动不再执行。日志文件命名为 `xdownload.log.YYYY-MM-DD`
/// （见 [`LocalDailyWriter`]），从文件名解析日期判断是否过期。
pub fn cleanup_old_logs() {
    // 仅每天第一次启动执行：marker 记录上次清理日期。
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let marker = crate::utils::app_home::AppHome::config_dir().join(LOG_CLEANUP_MARKER);
    if std::fs::read_to_string(&marker).ok() == Some(today.clone()) {
        return;
    }

    let removed = remove_old_logs(
        &crate::utils::app_home::AppHome::logs_dir(),
        chrono::Local::now().date_naive(),
    );
    let _ = std::fs::write(&marker, today);
    tracing::info!("cleaned old log files (>{LOG_RETENTION_DAYS} days): removed {removed}");
}

/// 删除 `logs_dir` 中早于 `now - LOG_RETENTION_DAYS` 天的日志文件，
/// 返回删除数量。日志文件命名为 `xdownload.log.YYYY-MM-DD`，从文件名
/// 解析日期；解析失败的文件跳过（不误删无关文件）。
fn remove_old_logs(logs_dir: &std::path::Path, today: chrono::NaiveDate) -> usize {
    let cutoff = today - chrono::Days::new(LOG_RETENTION_DAYS as u64);
    let mut removed = 0usize;
    if let Ok(entries) = std::fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            // 匹配 xdownload.log.YYYY-MM-DD
            let date_str = name.strip_prefix("xdownload.log.").unwrap_or(name);
            if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                if d < cutoff {
                    if std::fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
    }
    removed
}

/// Initialize the application

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

/// Enable virtual-terminal processing for the stdout console so the ANSI
/// colors emitted by the tracing console layer are parsed by cmd/conhost
/// (which otherwise prints the raw escape codes as mojibake). Debug builds are
/// CONSOLE-subsystem; release builds have no console and this is a harmless
/// no-op (GetConsoleMode fails on a non-console handle and we bail out).
#[cfg(windows)]
fn enable_vt_processing_on_stdout() {
    use windows::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE) else {
            return;
        };
        if handle.is_invalid() {
            return;
        }
        let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);
        if GetConsoleMode(handle, &mut mode).is_err() {
            return;
        }
        let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}

/// No-op on non-Windows platforms.
#[cfg(not(windows))]
fn enable_vt_processing_on_stdout() {}

/// Initialize the application
pub fn run() {
    // 在输出任何颜色日志前开启控制台 VT 处理（否则 ANSI 颜色码原样打印成乱码）。
    enable_vt_processing_on_stdout();

    // Initialize logging: rotate by LOCAL date and stamp timestamps in the
    // local timezone (previously UTC, off by up to a day / 8 hours). Logs live
    // in logs/ (kept out of config/, and gitignored as a whole).
    //
    // 双写：同一份日志同时写入 logs/ 文件与 stdout。debug 构建（CONSOLE
    // 子系统）会在黑色控制台实时滚动，便于开发调试；release 构建无控制台，
    // stdout layer 自然没有输出目标，正式版表现与之前完全一致。
    let _ = utils::app_home::AppHome::ensure_logs_dir();
    let (non_blocking, _guard) = tracing_appender::non_blocking(LocalDailyWriter::new(
        utils::app_home::AppHome::logs_dir(),
    ));

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTimer)
        .with_writer(non_blocking);
    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTimer)
        .with_writer(std::io::stdout);
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    // Load saved language setting
    services::i18n::I18n::load_saved();

    // Initialize from environment variables first (lowest priority)
    services::proxy::ProxyConfig::init_from_environment();

    // Detect system proxy (higher priority than env vars, lower than saved)
    services::proxy::ProxyConfig::detect_system_proxy();

    // Apply saved proxy (highest priority — overrides env and system detection)
    services::config::ConfigManager::apply_saved_proxy();

    // Clean stale download cache entries: dirs untouched for > 7 days
    // (abandoned tasks) are removed. Live `.part` files are kept so
    // interrupted downloads can be resumed across sessions.
    YtDlpDownloader::cleanup_download_cache();

    // Clean old log files (once per day, first launch). Logs accumulate
    // indefinitely otherwise; keep the last 14 days.
    cleanup_old_logs();

    // Initialize the download history database (config/data.db). The legacy
    // config/downloads.json is not migrated — history lives in SQLite only.
    services::download_history::DownloadHistory::init();

    // Create the downloader
    let downloader = Arc::new(YtDlpDownloader::new());

    // Apply saved cookies
    if let Some(ref b) = services::config::ConfigManager::load_cookie_source() {
        downloader.set_cookies_from_browser(b);
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
            commands::download::reorder_queue_task,
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
            commands::settings::load_cookie_source,
            commands::settings::save_cookie_source,
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
            commands::cookies::list_browsers,
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
            commands::settings::sync_bookmarks_preview,
            commands::settings::confirm_bookmarks_enqueue,
            commands::settings::list_bookmarks,
            commands::settings::get_version,
            commands::bootstrap::quit_app,
            commands::bootstrap::get_uninstall_info,
            commands::bootstrap::uninstall_app,
            commands::bootstrap::open_uninstall_panel,
            commands::update::check_ytdlp_update,
            commands::update::check_ffmpeg_update,
            commands::update::check_update_network,
            commands::update::cleanup_updater_temp,
            commands::history::list_download_history,
            commands::history::delete_download_history,
            commands::history::delete_download_history_file,
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

            // 书签同步改为手动触发（设置页「同步书签」按钮 → 预览弹窗 →
            // 用户确认后才入队），不再有后台轮询任务。

            // 深链批量合并 worker：短窗口收集 → 去重 → 并发 fetch → 批量入队。
            let batcher = DeepLinkBatcher::default();
            app.manage(batcher.clone());
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        batcher.notify.notified().await;
                        // 300ms 窗口：收集窗口期内密集到达的深链，合并成一批。
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        let targets: Vec<String> = {
                            let mut p = batcher.pending.lock().unwrap();
                            p.drain(..).collect()
                        };
                        if targets.is_empty() {
                            continue;
                        }
                        // 同 URL 只入队一次（保留首次出现顺序）。
                        let mut seen = HashSet::new();
                        let unique: Vec<String> = targets
                            .into_iter()
                            .filter(|t| seen.insert(t.clone()))
                            .collect();
                        process_deep_link_batch(&handle, &unique).await;
                    }
                });
            }

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

/// 深链批量合并缓冲：浏览器扩展快速连续点击多个视频时，多条
/// `xdownload://add?url=` 深链会密集到达。worker 收集一个短窗口内的
/// URL，去重后并发 fetch、批量入队，并只向前端发一条合并提示。
#[derive(Clone, Default)]
struct DeepLinkBatcher {
    /// 待处理的目标 URL（校验通过后先入缓冲）。
    pending: Arc<StdMutex<Vec<String>>>,
    /// 有新的深链到达时唤醒 worker。
    notify: Arc<tokio::sync::Notify>,
}

/// Validate a deep-link URL, then hand it to the batch worker for queuing.
/// The `setqueryid` action is handled first (queryId push, not a download).
fn handle_deep_link(app: &tauri::AppHandle, raw: &str) {
    // 扩展捕获的 queryId 推送：xdownload://setqueryid?value=<id> → 存 config 表。
    if let Some(qid) = parse_query_id_deep_link(raw) {
        match services::query_id::save(&qid) {
            Ok(_) => tracing::info!("deep-link: saved bookmarks queryId: {}", qid),
            Err(e) => tracing::warn!("deep-link: failed to save queryId: {e:#}"),
        }
        return;
    }

    let Some(target) = parse_deep_link_url(raw) else {
        tracing::warn!("deep-link: invalid url {}", raw);
        return;
    };
    if !commands::download::is_supported_url(&target) {
        tracing::warn!("deep-link: rejected non-X url {}", target);
        return;
    }
    tracing::info!("deep-link: received {}", target);
    let Some(batcher) = app.try_state::<DeepLinkBatcher>() else {
        return;
    };
    batcher.pending.lock().unwrap().push(target);
    batcher.notify.notify_one();
}

/// 批量处理一批深链 URL：并发 fetch 视频信息、逐条入队、合并上报前端。
async fn process_deep_link_batch(handle: &tauri::AppHandle, targets: &[String]) {
    let Some(state) = handle.try_state::<commands::download::DownloaderState>() else {
        return;
    };
    // 绝对路径兜底：协议拉起时进程 cwd 可能是 system32，相对路径
    // "downloads" 会解析到错误位置，必须用安装目录下的 downloads。
    let output_dir = services::config::ConfigManager::load_download_dir()
        .unwrap_or_else(|| {
            utils::app_home::AppHome::downloads_dir()
                .to_string_lossy()
                .to_string()
        });
    let downloader = state.downloader.clone();
    let queue = state.queue.clone();

    // 并发 fetch（join_all），显著缩短连续点击多条时的总等待。
    let results = futures_util::future::join_all(targets.iter().map(|target| {
        let downloader = downloader.clone();
        async move {
            // 从 status URL 提取 id 填入 video_id：历史记录以 video_id 为键，
            // 缺失会导致下载完成后不写下载历史。
            let video_id = commands::download::extract_status_id(target);
            // fetch 失败仍入队（不带 info，下载照常，yt-dlp 下载时会自行解析）。
            let fetch = downloader.fetch_video_info(target).await;
            let (title, info) = match fetch {
                Ok(info) => {
                    tracing::info!("deep-link: info fetched for {}", target);
                    (info.title.clone(), serde_json::to_value(&info).ok())
                }
                Err(e) => {
                    tracing::warn!("deep-link: fetch info failed: {}", e);
                    (None, None)
                }
            };
            (target.clone(), video_id, title, info)
        }
    }))
    .await;

    let mut added = 0usize;
    for (target, video_id, title, info) in results {
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
            output_dir: output_dir.clone(),
            output_template: "%(title)s.%(ext)s".to_string(),
            extract_audio: false,
            embed_subtitles: false,
            embed_thumbnail: false,
            write_thumbnail: false,
            proxy: None,
            socket_timeout: 30,
            cookies_from_browser: None,
            max_height: 0,
            download_archive: None,
            playlist_items: None,
            download_rate_limit: services::config::ConfigManager::load()
                .download_rate_limit
                .clone(),
        };
        // 深链（浏览器扩展）批量入队 → 标记为「批量」来源。
        match queue.enqueue(
            config,
            title,
            true,
            info,
            crate::services::download_history::source::BATCH,
        ) {
            Ok(id) => {
                added += 1;
                tracing::info!("deep-link: enqueued task {} for {}", id, target);
            }
            Err(e) if e.to_string() == "链接已在队列中" => {
                tracing::info!("deep-link: already in queue: {}", target);
            }
            Err(e) => {
                tracing::warn!("deep-link: enqueue failed: {}", e);
            }
        }
    }
    if added > 0 {
        // 专用事件：告知前端「已从浏览器获得 N 个下载任务」（合并提示）。
        // 普通入队的 download-queued 不区分来源，这里单独发一个事件，
        // 前端据此弹出 toast 并跳转到任务页。
        let _ = handle.emit("deep-link-queued", serde_json::json!({ "count": added }));
    }
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

/// Parse a queryId deep-link `xdownload://setqueryid?value=<encoded>`.
/// Returns the decoded and validated queryId, or `None` for other hosts /
/// missing / invalid values. The extension's popup triggers this deep link
/// after capturing a fresh id from live x.com traffic.
fn parse_query_id_deep_link(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("xdownload://")?;
    let (host, query) = rest.split_once('?')?;
    let host = host.trim_end_matches('/');
    if host != "setqueryid" {
        return None;
    }
    let mut value = None;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next()?;
        let v = it.next()?;
        if k == "value" {
            value = Some(percent_decode_str(v));
        }
    }
    value.filter(|v| services::query_id::is_valid_query_id(v))
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

    fn make_log_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "xdl_log_cleanup_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn removes_only_logs_older_than_retention() {
        let dir = make_log_dir("retention");
        // 4 个日志文件 + 1 个非日志文件。today = 2026-08-13，
        // cutoff = 08-13 - 14天 = 07-30（>= 07-30 保留，< 07-30 删除）。
        std::fs::write(dir.join("xdownload.log.2026-07-01"), "a").unwrap(); // 过期(<cutoff)
        std::fs::write(dir.join("xdownload.log.2026-07-29"), "b").unwrap(); // 过期(<cutoff)
        std::fs::write(dir.join("xdownload.log.2026-07-30"), "c").unwrap(); // 保留(==cutoff)
        std::fs::write(dir.join("xdownload.log.2026-08-10"), "d").unwrap(); // 保留
        std::fs::write(dir.join("readme.txt"), "not a log").unwrap(); // 不删

        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        let removed = remove_old_logs(&dir, today);
        assert_eq!(removed, 2);
        assert!(!dir.join("xdownload.log.2026-07-01").exists());
        assert!(!dir.join("xdownload.log.2026-07-29").exists());
        assert!(dir.join("xdownload.log.2026-07-30").exists());
        assert!(dir.join("xdownload.log.2026-08-10").exists());
        assert!(dir.join("readme.txt").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_all_when_none_are_old() {
        let dir = make_log_dir("fresh");
        std::fs::write(dir.join("xdownload.log.2026-08-13"), "a").unwrap();
        std::fs::write(dir.join("xdownload.log.2026-08-01"), "b").unwrap();

        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        let removed = remove_old_logs(&dir, today);
        assert_eq!(removed, 0);
        assert!(dir.join("xdownload.log.2026-08-13").exists());
        assert!(dir.join("xdownload.log.2026-08-01").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

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
    fn test_parse_query_id_deep_link() {
        // 标准形态。
        let v = parse_query_id_deep_link("xdownload://setqueryid?value=iblrFnKr6PZUR-dWpfXG6g");
        assert_eq!(v.as_deref(), Some("iblrFnKr6PZUR-dWpfXG6g"));
        // popup 用 encodeURIComponent，'%69...' 应解码。
        let v = parse_query_id_deep_link("xdownload://setqueryid?value=%69blrFnKr6PZUR-dWpfXG6g");
        assert_eq!(v.as_deref(), Some("iblrFnKr6PZUR-dWpfXG6g"));
        // 带尾斜杠的 host。
        let v = parse_query_id_deep_link("xdownload://setqueryid/?value=abc123_-");
        assert_eq!(v.as_deref(), Some("abc123_-"));
        // 非 setqueryid host → None（不干扰 add）。
        assert_eq!(
            parse_query_id_deep_link("xdownload://add?url=https://x.com/a"),
            None
        );
        // 缺 value 参数 → None。
        assert_eq!(parse_query_id_deep_link("xdownload://setqueryid?foo=bar"), None);
        assert_eq!(parse_query_id_deep_link("xdownload://setqueryid"), None);
        // 非法字符（空格/中文）→ None。
        assert_eq!(
            parse_query_id_deep_link("xdownload://setqueryid?value=has%20space"),
            None
        );
        assert_eq!(
            parse_query_id_deep_link("xdownload://setqueryid?value=%E6%88%91"),
            None
        );
        // 超长 → None。
        let long = format!("xdownload://setqueryid?value={}", "a".repeat(65));
        assert_eq!(parse_query_id_deep_link(&long), None);
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
