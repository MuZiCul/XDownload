use crate::downloader::ytdlp::YtDlpDownloader;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;

/// State container for the downloader
pub struct DownloaderState {
    pub downloader: Arc<YtDlpDownloader>,
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

    state
        .downloader
        .fetch_video_info(&url)
        .await
        .map_err(|e| e.to_string())
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

    if result {
        let _ = app.emit("download-complete", &config.url);
    } else {
        let _ = app.emit("download-error", "下载失败");
    }

    Ok(result)
}

/// Cancel the current download
#[tauri::command]
pub fn cancel_download(state: tauri::State<'_, DownloaderState>) {
    state.downloader.cancel();
}
