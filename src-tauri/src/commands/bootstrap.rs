use tauri::{AppHandle, Emitter};

/// Check if Google is reachable (used before tool downloads)
#[tauri::command]
pub async fn ping_google() -> bool {
    crate::services::network::NetworkDetect::is_google_accessible().await
}

/// Cancel the current bootstrap download
#[tauri::command]
pub fn cancel_bootstrap_download() {
    crate::services::bootstrap::cancel_download();
}

/// Get the bin directory path where yt-dlp and ffmpeg are stored
#[tauri::command]
pub fn get_bin_dir() -> String {
    crate::utils::app_home::AppHome::bin_dir()
        .to_string_lossy()
        .to_string()
}

/// Check if yt-dlp is available and get its version (single call)
#[tauri::command]
pub async fn check_ytdlp() -> serde_json::Value {
    let ytdlp = crate::utils::process::find_ytdlp();
    let ytdlp_str = ytdlp.to_str().unwrap_or("yt-dlp");

    match crate::utils::process::execute_with_timeout(&[ytdlp_str, "--version"], 5).await {
        Ok(result) if result.is_success() && !result.stdout.is_empty() => {
            let version = result.stdout[0].trim().to_string();
            serde_json::json!({ "available": true, "version": version })
        }
        _ => {
            serde_json::json!({ "available": false, "version": null })
        }
    }
}

/// Check if ffmpeg is available and get its version
#[tauri::command]
pub async fn check_ffmpeg() -> serde_json::Value {
    let ffmpeg = crate::utils::process::find_ffmpeg();
    if !ffmpeg.exists() {
        return serde_json::json!({ "available": false, "version": Option::<String>::None });
    }

    let ffmpeg_str = ffmpeg.to_str().unwrap_or("ffmpeg");
    match crate::utils::process::execute_with_timeout(&[ffmpeg_str, "-version"], 5).await {
        Ok(result) if result.is_success() && !result.stdout.is_empty() => {
            // Parse "ffmpeg version 7.1-essentials_build-..." → "7.1"
            let version = crate::commands::update::parse_ffmpeg_version_export(&result.stdout[0]);
            serde_json::json!({ "available": true, "version": version })
        }
        _ => {
            serde_json::json!({ "available": false, "version": Option::<String>::None })
        }
    }
}

/// Download yt-dlp with progress events
#[tauri::command]
pub async fn download_ytdlp(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();
    let result = crate::services::bootstrap::Bootstrap::download_ytdlp(move |pct| {
        let _ = app_clone.emit("bootstrap-progress", serde_json::json!({
            "tool": "yt-dlp",
            "percent": pct,
        }));
    })
    .await;

    match result {
        Ok(path) => {
            let _ = app.emit("bootstrap-complete", serde_json::json!({
                "tool": "yt-dlp",
                "success": true,
            }));
            Ok(path.to_string_lossy().to_string())
        }
        Err(e) => {
            let _ = app.emit("bootstrap-complete", serde_json::json!({
                "tool": "yt-dlp",
                "success": false,
            }));
            Err(format!("failed: {}", e))
        }
    }
}

/// Download ffmpeg with progress events
#[tauri::command]
pub async fn download_ffmpeg(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();
    let result = crate::services::bootstrap::Bootstrap::download_ffmpeg(
        {
            let app = app.clone();
            move |pct| {
                let _ = app.emit("bootstrap-progress", serde_json::json!({
                    "tool": "ffmpeg",
                    "percent": pct,
                    "stage": "downloading",
                }));
            }
        },
        move || {
            let _ = app_clone.emit("bootstrap-progress", serde_json::json!({
                "tool": "ffmpeg",
                "percent": 100,
                "stage": "extracting",
            }));
        },
    )
    .await;

    match result {
        Ok(path) => {
            let _ = app.emit("bootstrap-complete", serde_json::json!({
                "tool": "ffmpeg",
                "success": true,
            }));
            Ok(path.to_string_lossy().to_string())
        }
        Err(e) => {
            let _ = app.emit("bootstrap-complete", serde_json::json!({
                "tool": "ffmpeg",
                "success": false,
            }));
            Err(format!("failed: {}", e))
        }
    }
}
