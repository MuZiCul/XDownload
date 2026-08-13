//! Commands for the download-history page (list / delete / clear).

use crate::services::download_history::DownloadHistory;
use std::path::Path;

/// Resolve a possibly-relative history path against the current directory so
/// old records (saved with a relative `downloads\…`) still point at the real
/// absolute file.
pub fn abs_history_path(p: &str) -> String {
    let path = Path::new(p);
    if path.is_absolute() {
        p.to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(path)
            .to_string_lossy()
            .to_string()
    }
}

/// List all download history records (most recent first), with a flag telling
/// whether the saved file still exists on disk.
#[tauri::command]
pub fn list_download_history() -> Vec<serde_json::Value> {
    DownloadHistory::list()
        .into_iter()
        .map(|rec| {
            // 兼容旧记录：相对路径转绝对路径返回（供 opener 播放/打开）。
            let file_path = rec.file_path.as_deref().map(abs_history_path);
            let file_exists = file_path
                .as_ref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false);
            serde_json::json!({
                "video_id": rec.video_id,
                "title": rec.title,
                "thumbnail": rec.thumbnail,
                "url": rec.url,
                "uploader": rec.uploader,
                "duration": rec.duration,
                "view_count": rec.view_count,
                "like_count": rec.like_count,
                "file_path": file_path,
                "file_size": rec.file_size,
                "downloaded_at": rec.downloaded_at,
                "file_exists": file_exists,
                "status": rec.status,
                "error": rec.error,
                "attempts": rec.attempts,
            })
        })
        .collect()
}

/// Delete a single download-history record by video id.
#[tauri::command]
pub fn delete_download_history(video_id: String) -> Result<(), String> {
    DownloadHistory::remove(&video_id).map_err(|e| e.to_string())
}

/// Delete a single download-history record, optionally also deleting the
/// downloaded file on disk (resolved to an absolute path first).
#[tauri::command]
pub fn delete_download_history_file(
    video_id: String,
    delete_file: bool,
) -> Result<(), String> {
    if delete_file {
        if let Some(rec) = DownloadHistory::get(&video_id) {
            if let Some(p) = rec.file_path.as_deref().map(abs_history_path) {
                let path = Path::new(&p);
                if path.exists() {
                    std::fs::remove_file(path).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    DownloadHistory::remove(&video_id).map_err(|e| e.to_string())
}

/// Clear all download history.
#[tauri::command]
pub fn clear_download_history() -> Result<(), String> {
    DownloadHistory::clear().map_err(|e| e.to_string())
}
