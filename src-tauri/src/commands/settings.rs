use crate::models::config::AppSettings;
use crate::services::config::ConfigManager;
use tauri::Emitter;
use tracing::info;

/// Load settings from the active config file (config/settings.json).
#[tauri::command]
pub fn load_settings() -> AppSettings {
    ConfigManager::load()
}

/// Save settings to the active config file.
#[tauri::command]
pub fn save_settings(
    settings: AppSettings,
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<(), String> {
    // 记录开关类字段的变更（队列持久化 / 下载时防休眠），方便排查。
    let prev = ConfigManager::load();
    let qp_old = prev.queue_persist.unwrap_or(false);
    let qp_new = settings.queue_persist.unwrap_or(false);
    if qp_old != qp_new {
        info!("saving queue_persist: {} -> {}", qp_old, qp_new);
    }
    let ka_old = prev.keep_awake.unwrap_or(false);
    let ka_new = settings.keep_awake.unwrap_or(false);
    if ka_old != ka_new {
        info!("saving keep_awake: {} -> {}", ka_old, ka_new);
    }

    ConfigManager::save(&settings).map_err(|e| e.to_string())?;
    // 防休眠开关变化立即生效：sync 内部读取刚落盘的 keep_awake 配置。
    state.queue.sync_keep_awake();
    Ok(())
}

/// Get the download directory (saved or default) as an ABSOLUTE path.
/// Relative / empty / missing configs resolve against the app root
/// (`<root>/downloads`), so the settings page always shows a real path.
#[tauri::command]
pub fn get_download_dir() -> String {
    match ConfigManager::load_download_dir() {
        Some(d) if !d.is_empty() && std::path::Path::new(&d).is_absolute() => d,
        _ => crate::utils::app_home::AppHome::downloads_dir()
            .to_string_lossy()
            .into_owned(),
    }
}

/// Load the saved cookie source (browser name) from config.
#[tauri::command]
pub fn load_cookie_source() -> Option<String> {
    ConfigManager::load_cookie_source()
}

/// Save the cookie source (browser name) and apply it to the downloader.
#[tauri::command]
pub fn save_cookie_source(
    browser: Option<String>,
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<(), String> {
    ConfigManager::save_cookie_source(browser.as_deref())
        .map_err(|e| e.to_string())?;
    if let Some(ref b) = browser {
        if !b.is_empty() {
            state.downloader.set_cookies_from_browser(b);
        }
    }
    Ok(())
}

/// Save language only (merge into active config).
#[tauri::command]
pub fn save_language(lang: String) -> Result<(), String> {
    ConfigManager::save_lang(&lang).map_err(|e| e.to_string())
}

/// Get the app version. Version number is a single source of truth in
/// `Cargo.toml` (`CARGO_PKG_VERSION`), injected at compile time.
#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the persisted privacy mode state.
#[tauri::command]
pub fn get_privacy_mode() -> bool {
    ConfigManager::load_privacy_mode()
}

/// Persist the privacy mode state.
#[tauri::command]
pub fn set_privacy_mode(enabled: bool) -> Result<(), String> {
    ConfigManager::save_privacy_mode(enabled).map_err(|e| e.to_string())
}

/// 即时持久化「工具下载默认走代理」开关状态。
#[tauri::command]
pub fn set_tools_use_proxy(enabled: bool) -> Result<(), String> {
    ConfigManager::save_tools_use_proxy(enabled).map_err(|e| e.to_string())
}

/// Get whether the user has accepted the disclaimer on first launch.
/// Returns `false` when the field is missing (never accepted / old config).
#[tauri::command]
pub fn get_disclaimer_accepted() -> bool {
    ConfigManager::is_disclaimer_accepted()
}

/// Mark the disclaimer as accepted and persist to active config.
#[tauri::command]
pub fn accept_disclaimer() -> Result<(), String> {
    ConfigManager::accept_disclaimer().map_err(|e| e.to_string())
}

/// Stage 1 of the manual bookmarks sync: fetch all bookmarks and diff against
/// the download history (the sync cursor). Returns what a sync would find
/// (total / new count / the video-bearing new bookmarks) WITHOUT touching the
/// queue. Emits `bookmark-sync-progress` events at each phase so the UI can
/// show a real progress step (the modal cannot be closed while syncing).
#[tauri::command]
pub async fn sync_bookmarks_preview(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let browser = crate::services::config::ConfigManager::load_cookie_source()
        .unwrap_or_default();
    tracing::info!("[cmd] sync_bookmarks_preview called, browser={browser}");
    // 每阶段向后端 emit 进度事件；前端全局模态据此显示步骤。
    let emit_step = |step: crate::services::bookmarks::BookmarkSyncStep| {
        let _ = app.emit("bookmark-sync-progress", serde_json::json!({ "step": step }));
    };
    match crate::services::bookmarks::fetch_bookmark_changes(&browser, emit_step).await {
        Ok(changes) => {
            let v = serde_json::to_value(&changes).map_err(|e| e.to_string())?;
            tracing::info!("[cmd] sync_bookmarks_preview ok");
            Ok(v)
        }
        Err(e) => {
            tracing::warn!("[cmd] sync_bookmarks_preview failed: {e}");
            Err(e)
        }
    }
}

/// Stage 2 of the manual bookmarks sync: enqueue the user-confirmed videos
/// into the download queue. Nothing is persisted here — the download history
/// acts as the sync cursor.
#[tauri::command]
pub async fn confirm_bookmarks_enqueue(
    items: Vec<crate::services::bookmarks::BookmarkItem>,
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<serde_json::Value, String> {
    let queue = state.queue.clone();
    let result = crate::services::bookmarks::confirm_bookmark_enqueue(&queue, items).await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

/// List the persisted bookmark catalogue (every bookmark seen during syncs,
/// video and non-video alike) with live download-state flags. Reads from the
/// `bookmarks` table, so it works offline without touching X.
#[tauri::command]
pub fn list_bookmarks() -> Vec<crate::services::bookmarks_store::BookmarkRow> {
    crate::services::bookmarks_store::list()
}


