use crate::models::config::AppSettings;
use crate::utils::app_home::AppHome;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

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
                info!(
                    "no default config, seeding {} with factory defaults",
                    Self::default_file().display()
                );
                Self::write_json(&Self::default_file(), &defaults).ok();
                defaults
            }
        }
    }

    /// Save current settings as the new default config.
    pub fn save_as_default(settings: &AppSettings) -> Result<()> {
        Self::ensure_dir()?;
        info!(
            "saving current settings as default config → {}",
            Self::default_file().display()
        );
        Self::write_json(&Self::default_file(), settings)
    }

    // ==================== Helpers ====================

    fn read_json(path: &std::path::Path) -> Result<AppSettings> {
        let json = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("config file {} does not exist yet, using defaults", path.display());
                return Err(anyhow::anyhow!(e).context(format!("config file not found: {}", path.display())));
            }
            Err(e) => {
                warn!("failed to read config {}: {}", path.display(), e);
                return Err(anyhow::anyhow!(e).context(format!("failed to read {}", path.display())));
            }
        };
        let cfg = match serde_json::from_str(&json) {
            Ok(c) => c,
            Err(e) => {
                warn!("failed to parse config {}: {}", path.display(), e);
                return Err(anyhow::anyhow!(e).context(format!("failed to parse {}", path.display())));
            }
        };
        debug!("read config {} ({} bytes)", path.display(), json.len());
        Ok(cfg)
    }

    fn write_json(path: &std::path::Path, settings: &AppSettings) -> Result<()> {
        let json = match serde_json::to_string_pretty(settings) {
            Ok(j) => j,
            Err(e) => {
                error!("failed to serialize settings: {}", e);
                return Err(e).context("failed to serialize settings");
            }
        };
        match std::fs::write(path, &json) {
            Ok(_) => {}
            Err(e) => {
                error!("failed to write config {}: {}", path.display(), e);
                return Err(e).context(format!("failed to write {}", path.display()));
            }
        }
        debug!("wrote config {} ({} bytes)", path.display(), json.len());
        Ok(())
    }

    /// Merge the given settings into the active config (read-modify-write).
    fn merge_and_save(updater: impl FnOnce(&mut AppSettings)) -> Result<()> {
        let mut cfg = Self::load();
        updater(&mut cfg);
        Self::save(&cfg)
    }

    /// Public wrapper of [`Self::merge_and_save`] for cross-module use.
    pub fn merge_and_save_public(updater: impl FnOnce(&mut AppSettings)) -> Result<()> {
        Self::merge_and_save(updater)
    }

    // ==================== Proxy ====================

    pub fn save_proxy(host: &str, port: u32, scheme: &str) -> Result<()> {
        info!("saving proxy config: {}://{}:{}", scheme, host, port);
        Self::merge_and_save(|cfg| {
            cfg.proxy_host = Some(host.to_string());
            cfg.proxy_port = Some(port);
            cfg.proxy_scheme = Some(scheme.to_string());
        })
    }

    pub fn remove_proxy() -> Result<()> {
        info!("removing proxy config");
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

    pub fn save_cookie_source(browser: Option<&str>) -> Result<()> {
        info!("saving cookies config: browser={:?}", browser);
        Self::merge_and_save(|cfg| {
            if let Some(b) = browser {
                if !b.is_empty() {
                    cfg.cookies_from_browser = Some(b.to_string());
                }
            }
        })
    }

    pub fn clear_cookies() -> Result<()> {
        info!("clearing cookies config");
        Self::merge_and_save(|cfg| {
            cfg.cookies_from_browser = None;
        })
    }

    pub fn load_cookie_source() -> Option<String> {
        let cfg = Self::load();
        cfg.cookies_from_browser
            .filter(|b| !b.is_empty())
    }

    // ==================== Language ====================

    pub fn save_lang(lang: &str) -> Result<()> {
        info!("saving language preference: {}", lang);
        Self::merge_and_save(|cfg| {
            cfg.lang = Some(lang.to_string());
        })
    }

    pub fn load_lang() -> Option<String> {
        Self::load().lang
    }

    // ==================== Privacy Mode ====================

    pub fn save_privacy_mode(enabled: bool) -> Result<()> {
        info!("saving privacy mode: {}", enabled);
        Self::merge_and_save(|cfg| {
            cfg.privacy_mode = Some(enabled);
        })
    }

    // ==================== Tools Proxy ====================

    /// 工具（yt-dlp/ffmpeg）下载默认走代理开关（即时持久化）。
    pub fn save_tools_use_proxy(enabled: bool) -> Result<()> {
        info!("saving tools_use_proxy: {}", enabled);
        Self::merge_and_save(|cfg| {
            cfg.tools_use_proxy = Some(enabled);
        })
    }

    pub fn load_privacy_mode() -> bool {
        Self::load().privacy_mode.unwrap_or(false)
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
        info!("saving download directory: {}", if dir.is_empty() { "(reset to default)" } else { dir });
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
