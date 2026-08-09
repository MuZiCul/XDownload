use crate::commands::history::abs_history_path;
use crate::downloader::queue::DownloadQueue;
use crate::downloader::ytdlp::YtDlpDownloader;
use crate::models::config::DownloadConfig;
use crate::models::video_info::VideoInfo;
use crate::services::download_history::DownloadHistory;
use std::sync::Arc;

/// State container for the downloader and the multi-task queue.
pub struct DownloaderState {
    pub downloader: Arc<YtDlpDownloader>,
    pub queue: Arc<DownloadQueue>,
}

/// Whether the URL points to x.com / twitter.com (or a subdomain of either).
/// Accepts `x.com`, `twitter.com`, `www.x.com`, `mobile.twitter.com`, etc.
/// Rejects anything else (e.g. `evilx.com`) instead of the previous naive
/// `contains("x.com")` check that also wrongly blocked `twitter.com`.
pub(crate) fn is_supported_url(url: &str) -> bool {
    let trimmed = url.trim();
    // Strip the scheme.
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    // Take the host portion (everything before the first / ? or #).
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_x = host == "x.com" || host.ends_with(".x.com");
    let is_twitter = host == "twitter.com" || host.ends_with(".twitter.com");
    is_x || is_twitter
}

/// Common video file extensions, used to match downloaded files by title.
const VIDEO_EXTS: &[&str] = &[
    "mp4", "webm", "mkv", "flv", "mov", "avi", "ts", "m4a", "mp3", "opus",
];

/// Attach download status to parsed video info. The "已下载" decision is based
/// on the FILE SYSTEM only:
/// 1. History record exists → downloaded = whether that file still exists.
/// 2. No record (e.g. record deleted but file left on disk) → look for a file
///    in the download directory whose name starts with the (sanitized) title.
fn attach_download_status(info: &mut VideoInfo) {
    if info.id.is_empty() {
        return;
    }
    // 1) 先按 yt-dlp 解析出的 media id 查询（单任务路径的记录键）。
    if let Some(rec) = DownloadHistory::get(&info.id) {
        let path = rec.file_path.as_deref().map(abs_history_path);
        let exists = path
            .as_ref()
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false);
        info.downloaded = exists;
        info.downloaded_at = Some(rec.downloaded_at);
        info.download_path = path;
        return;
    }
    // 2) 查不到时再从输入 URL 提取 status id 兜底查询（批量/历史回填记录的键）。
    if let Some(status_id) = extract_status_id(info.url.as_str()) {
        if let Some(rec) = DownloadHistory::get(&status_id) {
            let path = rec.file_path.as_deref().map(abs_history_path);
            let exists = path
                .as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false);
            info.downloaded = exists;
            info.downloaded_at = Some(rec.downloaded_at);
            info.download_path = path;
            return;
        }
    }
    // 3) 无记录（记录被删但文件在盘）→ 按净化标题找文件。
    if let Some(title) = info.title.as_deref() {
        if let Some(path) = find_file_by_title(title) {
            info.downloaded = true;
            info.download_path = Some(path);
        }
    }
}

/// Extract the status (tweet) id from an x.com/twitter.com URL like
/// "https://x.com/user/status/1234567890123456789/video/1".
fn extract_status_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"/status/(\d+)").ok()?;
    re.captures(url).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

/// Search the download directory for a file whose name starts with the
/// sanitized video title (any video/audio extension).
fn find_file_by_title(title: &str) -> Option<String> {
    // The saved filename is the sanitized title + extension, e.g.
    // "title.mp4". Sanitize "title.mp4" then strip ".mp4" to get the prefix.
    let cleaned = DownloadHistory::sanitize_filename(&format!("{}.mp4", title));
    let prefix = cleaned.trim_end_matches(".mp4");

    let dir = match crate::services::config::ConfigManager::load().download_dir {
        Some(d) if !d.is_empty() && std::path::Path::new(&d).is_absolute() => {
            std::path::PathBuf::from(d)
        }
        _ => crate::utils::app_home::AppHome::downloads_dir(),
    };

    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_video = VIDEO_EXTS.iter().any(|e| {
            name.to_lowercase().ends_with(&format!(".{}", e))
        });
        if is_video && name.starts_with(prefix) {
            return Some(entry.path().to_string_lossy().to_string());
        }
    }
    None
}

/// Fetch video information from a URL
#[tauri::command]
pub async fn fetch_video_info(
    url: String,
    state: tauri::State<'_, DownloaderState>,
) -> Result<VideoInfo, String> {
    // Check yt-dlp exists
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        return Err("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp".to_string());
    }

    if !is_supported_url(&url) {
        return Err("仅支持 X/Twitter 视频链接".to_string());
    }

    // Pre-check x.com accessibility
    if !crate::services::network::NetworkDetect::is_x_accessible().await {
        return Err("无法访问 x.com，请检查网络或代理设置".to_string());
    }

    let mut info = state
        .downloader
        .fetch_video_info(&url)
        .await
        .map_err(|e| e.to_string())?;

    attach_download_status(&mut info);

    Ok(info)
}

/// Re-check on disk whether a video has already been downloaded.
/// Used right before starting a download, so that if the user deleted the
/// previously saved file, we download again without asking.
#[tauri::command]
pub fn check_video_downloaded(video_id: String) -> serde_json::Value {
    if video_id.is_empty() {
        return serde_json::json!({
            "downloaded": false,
            "downloaded_at": null,
            "file_path": null,
        });
    }

    match DownloadHistory::get(&video_id) {
        Some(rec) => {
            let exists = rec
                .file_path
                .as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false);
            serde_json::json!({
                "downloaded": exists,
                "downloaded_at": rec.downloaded_at,
                "file_path": rec.file_path,
            })
        }
        None => serde_json::json!({
            "downloaded": false,
            "downloaded_at": null,
            "file_path": null,
        }),
    }
}

// ==================== Multi-task queue ====================

/// Enqueue a download into the multi-task queue. Returns the task id.
/// The URL is deduplicated against queued / running tasks.
/// `auto_start`: false → the task only waits until `start_queue` is called
/// (batch mode); true → the queue starts draining immediately.
#[tauri::command]
pub fn enqueue_download(
    config: DownloadConfig,
    title: Option<String>,
    auto_start: bool,
    info: Option<serde_json::Value>,
    state: tauri::State<'_, DownloaderState>,
) -> Result<String, String> {
    if !is_supported_url(&config.url) {
        return Err("仅支持 X/Twitter 视频链接".to_string());
    }
    state.queue.enqueue(config, title, auto_start, info)
}

/// Start draining the multi-task queue (batch mode "开始任务").
#[tauri::command]
pub fn start_queue(state: tauri::State<'_, DownloaderState>) {
    state.queue.start();
}

/// Pause the multi-task queue — no new tasks start, running tasks finish.
#[tauri::command]
pub fn pause_queue(state: tauri::State<'_, DownloaderState>) {
    state.queue.pause();
}

/// Resume a paused multi-task queue.
#[tauri::command]
pub fn resume_queue(state: tauri::State<'_, DownloaderState>) {
    state.queue.resume();
}

/// Pause a single task (queued → paused; running → kill + keep cache).
#[tauri::command]
pub fn pause_queue_task(task_id: String, state: tauri::State<'_, DownloaderState>) {
    state.queue.pause_task(&task_id);
}

/// Resume a paused task (resumes download from the kept .part cache).
#[tauri::command]
pub fn resume_queue_task(task_id: String, state: tauri::State<'_, DownloaderState>) {
    state.queue.resume_task(&task_id);
}

/// Pause every active task (each emits `download-paused`).
#[tauri::command]
pub fn pause_all_tasks(state: tauri::State<'_, DownloaderState>) {
    state.queue.pause_all();
}

/// Resume every paused task.
#[tauri::command]
pub fn resume_all_tasks(state: tauri::State<'_, DownloaderState>) {
    state.queue.resume_all();
}

/// Cancel a queued / running multi-task download.
#[tauri::command]
pub fn cancel_queue_task(task_id: String, state: tauri::State<'_, DownloaderState>) {
    state.queue.cancel_task(&task_id);
}

/// Remove all tasks still waiting in the queue (running tasks finish).
#[tauri::command]
pub fn clear_download_queue(state: tauri::State<'_, DownloaderState>) {
    state.queue.clear_queued();
}

/// Cancel ALL active tasks (queued / paused / running). Finished downloads in
/// the history are not affected.
#[tauri::command]
pub fn cancel_all_tasks(state: tauri::State<'_, DownloaderState>) {
    state.queue.cancel_all();
}

/// Whether there are active tasks (queued / running / paused) — used by the
/// exit-confirmation flow.
#[tauri::command]
pub fn has_active_tasks(state: tauri::State<'_, DownloaderState>) -> bool {
    state.queue.has_active()
}

/// Snapshot of the multi-task queue (queued + running) for the frontend.
#[tauri::command]
pub fn queue_status(state: tauri::State<'_, DownloaderState>) -> Vec<serde_json::Value> {
    state.queue.status()
}

/// Update a task's card metadata (thumbnail / uploader / duration / …) so the
/// info fetched by the frontend is persisted with the task and survives a
/// "保存进度并退出" restart.
#[tauri::command]
pub fn update_task_info(
    task_id: String,
    info: Option<serde_json::Value>,
    state: tauri::State<'_, DownloaderState>,
) {
    state.queue.update_info(&task_id, info);
}
