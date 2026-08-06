use crate::models::config::AppSettings;
use crate::utils::app_home::AppHome;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Persistent configuration manager.
///
/// Two files:
/// - `config/settings.json` — **active config** (day-to-day, auto-persisted)
/// - `config/default.json` — **default config** (factory baseline for restore)
pub struct ConfigManager;

impl ConfigManager {
    /// Active config: config/settings.json
    fn active_file() -> PathBuf {
        AppHome::config_dir().join("settings.json")
    }

    /// Default config: config/default.json
    fn default_file() -> PathBuf {
        AppHome::config_dir().join("default.json")
    }

    /// Ensure the config directory exists.
    fn ensure_dir() -> Result<()> {
        AppHome::ensure_config_dir()
            .context("failed to create config directory")
    }

    // ==================== Active Config ====================

    /// Load active configuration.
    /// Returns default AppSettings if the file does not exist.
    pub fn load() -> AppSettings {
        Self::read_json(&Self::active_file()).unwrap_or_default()
    }

    /// Save to active config.
    pub fn save(settings: &AppSettings) -> Result<()> {
        Self::ensure_dir()?;
        Self::write_json(&Self::active_file(), settings)
    }

    /// Save to an arbitrary file path (export).
    pub fn save_to_path(settings: &AppSettings, path: &str) -> Result<()> {
        let path_obj = std::path::Path::new(path);
        if let Some(parent) = path_obj.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create parent directories for custom config path")?;
        }
        Self::write_json(path_obj, settings)
    }

    // ==================== Default Config ====================

    /// Load default configuration.
    /// If default.json doesn't exist, create it with factory defaults first.
    pub fn load_default() -> AppSettings {
        match Self::read_json(&Self::default_file()) {
            Ok(cfg) => cfg,
            Err(_) => {
                // First run — seed default.json with factory defaults
                let defaults = AppSettings::default();
                Self::ensure_dir().ok();
                Self::write_json(&Self::default_file(), &defaults).ok();
                defaults
            }
        }
    }

    /// Save current settings as the new default config.
    pub fn save_as_default(settings: &AppSettings) -> Result<()> {
        Self::ensure_dir()?;
        Self::write_json(&Self::default_file(), settings)
    }

    // ==================== Helpers ====================

    fn read_json(path: &std::path::Path) -> Result<AppSettings> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&json)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    fn write_json(path: &std::path::Path, settings: &AppSettings) -> Result<()> {
        let json = serde_json::to_string_pretty(settings)
            .context("failed to serialize settings")?;
        std::fs::write(path, json)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    /// Merge the given settings into the active config (read-modify-write).
    fn merge_and_save(updater: impl FnOnce(&mut AppSettings)) -> Result<()> {
        let mut cfg = Self::load();
        updater(&mut cfg);
        Self::save(&cfg)
    }

    // ==================== Proxy ====================

    pub fn save_proxy(host: &str, port: u32, scheme: &str) -> Result<()> {
        Self::merge_and_save(|cfg| {
            cfg.proxy_host = Some(host.to_string());
            cfg.proxy_port = Some(port);
            cfg.proxy_scheme = Some(scheme.to_string());
        })
    }

    pub fn remove_proxy() -> Result<()> {
        Self::merge_and_save(|cfg| {
            cfg.proxy_host = None;
            cfg.proxy_port = None;
            cfg.proxy_scheme = None;
        })
    }

    /// Load proxy from active config and apply to runtime.
    pub fn apply_saved_proxy() -> bool {
        if crate::services::proxy::ProxyConfig::is_from_system_proxy() {
            return false;
        }
        let cfg = Self::load();
        match (cfg.proxy_host, cfg.proxy_port) {
            (Some(host), Some(port)) if !host.is_empty() => {
                let scheme = cfg.proxy_scheme.as_deref().unwrap_or("http");
                crate::services::proxy::ProxyConfig::set_proxy_full(
                    &host,
                    port.min(65535) as u16,
                    scheme,
                );
                true
            }
            _ => false,
        }
    }

    // ==================== Cookies ====================

    pub fn save_cookies(browser: Option<&str>, cookies_file: Option<&str>) -> Result<()> {
        Self::merge_and_save(|cfg| {
            if let Some(b) = browser {
                if !b.is_empty() {
                    cfg.cookies_from_browser = Some(b.to_string());
                    cfg.cookies_file = None;
                    return;
                }
            }
            if let Some(f) = cookies_file {
                if !f.is_empty() {
                    cfg.cookies_file = Some(f.to_string());
                    cfg.cookies_from_browser = None;
                }
            }
        })
    }

    pub fn clear_cookies() -> Result<()> {
        Self::merge_and_save(|cfg| {
            cfg.cookies_from_browser = None;
            cfg.cookies_file = None;
        })
    }

    pub fn load_saved_cookies() -> (Option<String>, Option<String>) {
        let cfg = Self::load();
        if let Some(browser) = cfg.cookies_from_browser.filter(|b| !b.is_empty()) {
            (Some(browser), None)
        } else if let Some(file) = cfg.cookies_file.filter(|f| !f.is_empty()) {
            (None, Some(file))
        } else {
            (None, None)
        }
    }

    // ==================== Language ====================

    pub fn save_lang(lang: &str) -> Result<()> {
        Self::merge_and_save(|cfg| {
            cfg.lang = Some(lang.to_string());
        })
    }

    pub fn load_lang() -> Option<String> {
        Self::load().lang
    }

    // ==================== Disclaimer ====================
    //
    // The acceptance state is stored in the Windows Registry (HKCU) with an
    // HMAC-SHA256 signature (see services::disclaimer), so users cannot bypass
    // the forced disclaimer by editing config/settings.json. The legacy
    // `disclaimer_accepted` JSON field is intentionally NOT honored.

    /// Mark the disclaimer as accepted and persist a signed record.
    pub fn accept_disclaimer() -> Result<()> {
        crate::services::disclaimer::accept()
    }

    /// Whether the disclaimer has been accepted (registry + signature check).
    pub fn is_disclaimer_accepted() -> bool {
        crate::services::disclaimer::is_accepted()
    }

    // ==================== Download Dir ====================

    pub fn save_download_dir(dir: &str) -> Result<()> {
        Self::merge_and_save(|cfg| {
            if dir.is_empty() {
                cfg.download_dir = None;
            } else {
                cfg.download_dir = Some(dir.to_string());
            }
        })
    }

    pub fn load_download_dir() -> Option<String> {
        Self::load().download_dir
    }

    // ==================== Paths ====================

    pub fn active_config_path() -> PathBuf {
        Self::active_file()
    }

    pub fn default_config_path() -> PathBuf {
        Self::default_file()
    }
}
