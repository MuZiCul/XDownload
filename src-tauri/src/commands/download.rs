use crate::downloader::ytdlp::YtDlpDownloader;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use crate::services::download_history::DownloadHistory;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;

/// State container for the downloader
pub struct DownloaderState {
    pub downloader: Arc<YtDlpDownloader>,
}

/// Attach download-history status to parsed video info.
fn attach_download_status(info: &mut VideoInfo) {
    if info.id.is_empty() {
        return;
    }
    if let Some(rec) = DownloadHistory::get(&info.id) {
        let exists = rec
            .file_path
            .as_ref()
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false);
        info.downloaded = exists;
        info.downloaded_at = Some(rec.downloaded_at);
        info.download_path = rec.file_path;
    }
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

    if !url.contains("x.com") {
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

/// Start a download with progress events
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    config: DownloadConfig,
    state: tauri::State<'_, DownloaderState>,
) -> Result<bool, String> {
    // Check yt-dlp exists
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        return Err("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp".to_string());
    }

    // Check ffmpeg exists
    let ffmpeg_path = crate::utils::process::find_ffmpeg();
    if !ffmpeg_path.exists() && ffmpeg_path.to_str() != Some("ffmpeg") {
        return Err("ffmpeg 未安装，请先在设置页面的 Tools 中下载 ffmpeg".to_string());
    }

    let downloader = state.downloader.clone();

    let result = downloader
        .download(&config, {
            let app = app.clone();
            move |progress: DownloadProgress| {
                let _ = app.emit("download-progress", &progress);
            }
        })
        .await
        .map_err(|e| e.to_string())?;

    match result {
        // Successful download → record history, then notify the frontend.
        Some(saved_path) => {
            if let Some(id) = config.video_id.as_deref() {
                if !id.is_empty() {
                    tracing::info!(
                        "record download history: id={} path={:?}",
                        id,
                        saved_path
                    );
                    let _ = DownloadHistory::record(id, None, Some(saved_path));
                }
            }
            let _ = app.emit("download-complete", &config.url);
            Ok(true)
        }
        // Process finished unsuccessfully without stderr detail.
        None => {
            let _ = app.emit("download-error", "下载失败");
            Ok(false)
        }
    }
}

/// Cancel the current download
#[tauri::command]
pub fn cancel_download(state: tauri::State<'_, DownloaderState>) {
    state.downloader.cancel();
}
