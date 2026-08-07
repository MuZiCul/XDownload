//! Commands for the download-history page (list / delete / clear).

use crate::services::download_history::DownloadHistory;
use std::path::Path;

/// List all download history records (most recent first), with a flag telling
/// whether the saved file still exists on disk.
#[tauri::command]
pub fn list_download_history() -> Vec<serde_json::Value> {
    DownloadHistory::list()
        .into_iter()
        .map(|rec| {
            let file_exists = rec
                .file_path
                .as_ref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false);
            serde_json::json!({
                "id": rec.id,
                "title": rec.title,
                "file_path": rec.file_path,
                "downloaded_at": rec.downloaded_at,
                "file_exists": file_exists,
            })
        })
        .collect()
}

/// Delete a single download-history record by video id.
#[tauri::command]
pub fn delete_download_history(id: String) -> Result<(), String> {
    DownloadHistory::remove(&id).map_err(|e| e.to_string())
}

/// Clear all download history.
#[tauri::command]
pub fn clear_download_history() -> Result<(), String> {
    DownloadHistory::clear().map_err(|e| e.to_string())
}
