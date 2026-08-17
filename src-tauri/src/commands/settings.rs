use crate::models::config::AppSettings;
use crate::services::config::ConfigManager;
use tauri::Emitter;

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
    ConfigManager::save(&settings).map_err(|e| e.to_string())?;
    // 防休眠开关变化立即生效：sync 内部读取刚落盘的 keep_awake 配置。
    state.queue.sync_keep_awake();
    Ok(())
}

/// Export settings to a custom file path.
#[tauri::command]
pub fn save_settings_to_path(settings: AppSettings, path: String) -> Result<(), String> {
    ConfigManager::save_to_path(&settings, &path).map_err(|e| e.to_string())
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

/// Get the active config file path for display.
#[tauri::command]
pub fn get_config_path() -> String {
    ConfigManager::active_config_path()
        .to_string_lossy()
        .to_string()
}

/// Apply proxy from active config to runtime (read-only).
#[tauri::command]
pub fn apply_saved_proxy() -> bool {
    ConfigManager::apply_saved_proxy()
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

/// Load settings from an arbitrary file path.
#[tauri::command]
pub fn load_settings_from_path(path: String) -> Result<AppSettings, String> {
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("无法读取配置文件: {}", e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("配置文件格式错误: {}", e))
}

/// Apply settings to runtime AND persist to the active config file.
/// Used after importing an external config or restoring defaults.
#[tauri::command]
pub fn apply_and_persist_settings(
    settings: AppSettings,
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<(), String> {
    // Apply proxy to runtime
    if let (Some(ref host), Some(port)) = (&settings.proxy_host, settings.proxy_port) {
        if !host.is_empty() {
            let scheme = settings.proxy_scheme.as_deref().unwrap_or("http");
            crate::services::proxy::ProxyConfig::set_proxy_full(
                host,
                port.min(65535) as u16,
                scheme,
            );
        }
    }
    // Apply cookies to runtime
    if let Some(ref browser) = settings.cookies_from_browser {
        if !browser.is_empty() {
            state.downloader.set_cookies_from_browser(browser);
        }
    }
    // Apply language to runtime
    if let Some(ref lang) = settings.lang {
        crate::services::i18n::I18n::set_lang(lang);
    }

    // Persist to active config
    ConfigManager::save(&settings).map_err(|e| e.to_string())?;
    // 导入的配置可能改变防休眠开关 → 同步运行时状态。
    state.queue.sync_keep_awake();
    Ok(())
}

/// Load the default config (config/default.json), apply to runtime,
/// and persist to active config.  Used by "应用配置 → 默认目录".
#[tauri::command]
pub fn apply_default_config(
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<AppSettings, String> {
    let defaults = ConfigManager::load_default();

    // Apply to runtime
    if let (Some(ref host), Some(port)) = (&defaults.proxy_host, defaults.proxy_port) {
        if !host.is_empty() {
            let scheme = defaults.proxy_scheme.as_deref().unwrap_or("http");
            crate::services::proxy::ProxyConfig::set_proxy_full(
                host,
                port.min(65535) as u16,
                scheme,
            );
        }
    }
    if let Some(ref browser) = defaults.cookies_from_browser {
        if !browser.is_empty() {
            state.downloader.set_cookies_from_browser(browser);
        }
    }
    if let Some(ref lang) = defaults.lang {
        crate::services::i18n::I18n::set_lang(lang);
    }

    // Persist to active config
    ConfigManager::save(&defaults).map_err(|e| e.to_string())?;
    // 默认配置可能改变防休眠开关 → 同步运行时状态。
    state.queue.sync_keep_awake();

    Ok(defaults)
}

/// Save current settings as the new default config (config/default.json).
#[tauri::command]
pub fn save_as_default(settings: AppSettings) -> Result<(), String> {
    ConfigManager::save_as_default(&settings).map_err(|e| e.to_string())
}
