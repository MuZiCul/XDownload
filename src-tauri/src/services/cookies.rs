use crate::utils::app_home::AppHome;
use std::path::PathBuf;

/// Cookie browser detection and management.
/// Provides browser cookie DB paths for Windows and utilities
/// for copying locked Chrome cookie databases.
pub struct CookieManager;

/// Ordered list of supported browsers for cookie extraction.
pub const BROWSER_FALLBACK_ORDER: &[&str] = &["chrome", "firefox", "edge", "brave", "opera"];

impl CookieManager {
    // ==================== Browser Cookie DB Paths ====================

    /// Get the cookie database path for a given browser on Windows.
    /// Returns None if the browser is not supported or the path cannot be determined.
    pub fn browser_cookie_path(browser: &str) -> Option<PathBuf> {
        match browser.to_lowercase().as_str() {
            "chrome" => {
                let local_appdata = std::env::var("LOCALAPPDATA").ok()?;
                Some(PathBuf::from(local_appdata)
                    .join("Google")
                    .join("Chrome")
                    .join("User Data")
                    .join("Default")
                    .join("Network")
                    .join("Cookies"))
            }
            "firefox" => {
                let appdata = std::env::var("APPDATA").ok()?;
                let profiles_dir = PathBuf::from(&appdata)
                    .join("Mozilla")
                    .join("Firefox")
                    .join("Profiles");
                // Find the first profile directory containing a cookies.sqlite file.
                // Prefer *.default-release, then fall back to any profile.
                if profiles_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                        // First pass: look for *.default-release
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.ends_with(".default-release") {
                                let db = entry.path().join("cookies.sqlite");
                                if db.exists() {
                                    return Some(db);
                                }
                            }
                        }
                        // Second pass: any profile
                        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                            for entry in entries.flatten() {
                                if entry.path().is_dir() {
                                    let db = entry.path().join("cookies.sqlite");
                                    if db.exists() {
                                        return Some(db);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            "edge" => {
                let local_appdata = std::env::var("LOCALAPPDATA").ok()?;
                Some(PathBuf::from(local_appdata)
                    .join("Microsoft")
                    .join("Edge")
                    .join("User Data")
                    .join("Default")
                    .join("Network")
                    .join("Cookies"))
            }
            "brave" => {
                let local_appdata = std::env::var("LOCALAPPDATA").ok()?;
                Some(PathBuf::from(local_appdata)
                    .join("BraveSoftware")
                    .join("Brave-Browser")
                    .join("User Data")
                    .join("Default")
                    .join("Network")
                    .join("Cookies"))
            }
            "opera" => {
                let appdata = std::env::var("APPDATA").ok()?;
                Some(PathBuf::from(appdata)
                    .join("Opera Software")
                    .join("Opera Stable")
                    .join("Default")
                    .join("Network")
                    .join("Cookies"))
            }
            _ => None,
        }
    }

    // ==================== Chrome Cookie DB Backup ====================

    /// Copy Chrome's Cookies database to config/ directory.
    /// This bypasses the file-lock issue when Chrome is running because
    /// on Windows, the file can still be read via NIO-style copy.
    /// Returns the backup path on success, None on failure.
    pub fn backup_chrome_cookies_db() -> Option<PathBuf> {
        let chrome_db = match Self::browser_cookie_path("chrome") {
            Some(p) if p.exists() => p,
            _ => return None,
        };

        let config_dir = AppHome::config_dir();
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            tracing::warn!("failed to create config dir for cookie backup: {}", e);
            return None;
        }

        let backup_path = config_dir.join("chrome_cookies_backup.db");

        // Retry up to 3 times with 1-second delay (Chrome may briefly hold a write lock)
        for attempt in 0..3 {
            match std::fs::copy(&chrome_db, &backup_path) {
                Ok(_) => return Some(backup_path),
                Err(e) => {
                    if attempt < 2 {
                        tracing::debug!(
                            "chrome cookie copy attempt {} failed: {}, retrying...",
                            attempt + 1,
                            e
                        );
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            }
        }

        None
    }

    /// Get the backup cookie database path (may not exist).
    pub fn backup_cookie_path() -> PathBuf {
        AppHome::config_dir().join("chrome_cookies_backup.db")
    }

    /// Check whether the backup is still valid (less than 1 hour old).
    pub fn is_backup_valid() -> bool {
        let backup = Self::backup_cookie_path();
        if !backup.exists() {
            return false;
        }
        if let Ok(meta) = std::fs::metadata(&backup) {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    return elapsed.as_secs() < 3600; // 1 hour
                }
            }
        }
        false
    }

    // ==================== Error Detection ====================

    /// Check if stderr output indicates a Chrome cookie database lock error.
    /// yt-dlp stderr typically contains "could not copy" when Chrome has the DB locked.
    pub fn is_chrome_lock_error(stderr: &str) -> bool {
        let lower = stderr.to_lowercase();
        (lower.contains("could not copy") && lower.contains("chrome"))
            || (lower.contains("could not copy") && lower.contains("cookie database"))
    }

    // ==================== Browser Scanning ====================

    /// Scan available browsers in fallback order and return the first one
    /// whose cookie database exists on disk.
    pub fn scan_available_browser() -> Option<String> {
        for browser in BROWSER_FALLBACK_ORDER {
            if let Some(path) = Self::browser_cookie_path(browser) {
                if path.exists() {
                    return Some(browser.to_string());
                }
            }
        }
        None
    }

    /// Get a list of all browsers whose cookie DB is present.
    pub fn list_available_browsers() -> Vec<String> {
        BROWSER_FALLBACK_ORDER
            .iter()
            .filter_map(|b| {
                Self::browser_cookie_path(b)
                    .filter(|p| p.exists())
                    .map(|_| b.to_string())
            })
            .collect()
    }
}
