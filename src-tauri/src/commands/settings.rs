use crate::models::config::AppSettings;
use crate::services::config::ConfigManager;

/// Load settings from the active config file (config/settings.json).
#[tauri::command]
pub fn load_settings() -> AppSettings {
    ConfigManager::load()
}

/// Save settings to the active config file.
#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    ConfigManager::save(&settings).map_err(|e| e.to_string())
}

/// Export settings to a custom file path.
#[tauri::command]
pub fn save_settings_to_path(settings: AppSettings, path: String) -> Result<(), String> {
    ConfigManager::save_to_path(&settings, &path).map_err(|e| e.to_string())
}

/// Get the download directory (saved or default).
#[tauri::command]
pub fn get_download_dir() -> String {
    ConfigManager::load_download_dir()
        .unwrap_or_else(|| "downloads".to_string())
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

/// Load saved cookies config (does not apply to downloader).
#[tauri::command]
pub fn load_saved_cookies() -> (Option<String>, Option<String>) {
    ConfigManager::load_saved_cookies()
}

/// Load cookies from active config and apply to downloader (read-only).
#[tauri::command]
pub fn apply_saved_cookies(
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<(), String> {
    let (browser, file) = ConfigManager::load_saved_cookies();
    if let Some(ref b) = browser {
        state.downloader.set_cookies_from_browser(b);
    } else if let Some(ref f) = file {
        state.downloader.set_cookies_file(f);
    } else {
        state.downloader.set_cookies_from_browser("");
    }
    Ok(())
}

/// Save cookies selection and apply to downloader.
#[tauri::command]
pub fn save_and_apply_cookies(
    browser: Option<String>,
    state: tauri::State<'_, crate::commands::download::DownloaderState>,
) -> Result<(), String> {
    ConfigManager::save_cookies(browser.as_deref(), None::<&str>)
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
            crate::services::proxy::ProxyConfig::set_proxy(host, port.min(65535) as u16);
        }
    }
    // Apply cookies to runtime
    if let Some(ref browser) = settings.cookies_from_browser {
        if !browser.is_empty() {
            state.downloader.set_cookies_from_browser(browser);
        }
    } else if let Some(ref file) = settings.cookies_file {
        if !file.is_empty() {
            state.downloader.set_cookies_file(file);
        }
    }
    // Apply language to runtime
    if let Some(ref lang) = settings.lang {
        crate::services::i18n::I18n::set_lang(lang);
    }

    // Persist to active config
    ConfigManager::save(&settings).map_err(|e| e.to_string())
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
            crate::services::proxy::ProxyConfig::set_proxy(host, port.min(65535) as u16);
        }
    }
    if let Some(ref browser) = defaults.cookies_from_browser {
        if !browser.is_empty() {
            state.downloader.set_cookies_from_browser(browser);
        }
    } else if let Some(ref file) = defaults.cookies_file {
        if !file.is_empty() {
            state.downloader.set_cookies_file(file);
        }
    }
    if let Some(ref lang) = defaults.lang {
        crate::services::i18n::I18n::set_lang(lang);
    }

    // Persist to active config
    ConfigManager::save(&defaults).map_err(|e| e.to_string())?;

    Ok(defaults)
}

/// Save current settings as the new default config (config/default.json).
#[tauri::command]
pub fn save_as_default(settings: AppSettings) -> Result<(), String> {
    ConfigManager::save_as_default(&settings).map_err(|e| e.to_string())
}
