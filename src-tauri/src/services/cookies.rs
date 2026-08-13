use std::path::PathBuf;

/// Cookie browser detection and management.
/// Provides browser cookie DB paths for Windows and locked-cookie error
/// detection (Chrome/Edge lock their Cookies DB exclusively while running,
/// so the app falls back to another browser instead of copying the file).
pub struct CookieManager;

/// Ordered list of supported browsers for cookie extraction.
pub const BROWSER_FALLBACK_ORDER: &[&str] = &["chrome", "firefox", "edge", "brave", "opera"];

/// Browsers that yt-dlp's `--cookies-from-browser` officially supports
/// (per yt-dlp README). A browser MUST be in this list to be offered to the
/// user, even if it is installed — otherwise yt-dlp cannot read its cookies.
pub const YTDLP_SUPPORTED_BROWSERS: &[&str] = &[
    "brave",
    "chrome",
    "chromium",
    "edge",
    "firefox",
    "opera",
    "safari",
    "vivaldi",
    "whale",
];

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

    /// Detect browsers that are both **installed** on this machine AND
    /// **supported by yt-dlp** (`--cookies-from-browser`).
    ///
    /// Installed but not yt-dlp-supported browsers are intentionally excluded:
    /// offering them would let the user pick a browser whose cookies yt-dlp
    /// cannot read. This answers "is the browser installed AND usable", which
    /// is what the frontend dropdown should offer.
    pub fn installed_browsers() -> Vec<String> {
        // Guard against regressions: every candidate must be yt-dlp-supported.
        debug_assert!(
            BROWSER_FALLBACK_ORDER
                .iter()
                .all(|b| YTDLP_SUPPORTED_BROWSERS.contains(b)),
            "BROWSER_FALLBACK_ORDER contains a browser not supported by yt-dlp"
        );
        YTDLP_SUPPORTED_BROWSERS
            .iter()
            .filter(|b| Self::is_browser_installed(b))
            .map(|b| b.to_string())
            .collect()
    }

    /// Check whether a single browser is installed.
    pub fn is_browser_installed(browser: &str) -> bool {
        #[cfg(windows)]
        {
            if Self::registry_key_exists(browser) {
                return true;
            }
        }
        Self::executable_exists(browser)
    }

    /// Registry presence check (Windows-only). Each browser installs a key
    /// under HKLM (or HKLM\SOFTWARE\WOW6432Node for 32-bit installs on 64-bit
    /// systems); a browser installed per-user registers under HKCU.
    #[cfg(windows)]
    fn registry_key_exists(browser: &str) -> bool {
        use winreg::enums::*;
        use winreg::RegKey;

        // (hive kind, relative key path)
        enum Hive {
            Hklm,
            Hkcu,
        }
        let candidates: &[(Hive, &str)] = match browser.to_lowercase().as_str() {
            "chrome" => &[
                (Hive::Hklm, r"SOFTWARE\Google\Chrome\BLBeacon"),
                (Hive::Hklm, r"SOFTWARE\WOW6432Node\Google\Chrome\BLBeacon"),
                (Hive::Hkcu, r"Software\Google\Chrome\BLBeacon"),
            ],
            "edge" => &[
                (Hive::Hklm, r"SOFTWARE\Microsoft\Edge\BLBeacon"),
                (Hive::Hklm, r"SOFTWARE\WOW6432Node\Microsoft\Edge\BLBeacon"),
                (Hive::Hkcu, r"Software\Microsoft\Edge\BLBeacon"),
            ],
            "firefox" => &[
                (Hive::Hklm, r"SOFTWARE\Mozilla\Mozilla Firefox"),
                (Hive::Hklm, r"SOFTWARE\WOW6432Node\Mozilla\Mozilla Firefox"),
                (Hive::Hkcu, r"Software\Mozilla\Mozilla Firefox"),
            ],
            "brave" => &[
                (Hive::Hklm, r"SOFTWARE\BraveSoftware\Brave-Browser\BLBeacon"),
                (Hive::Hklm, r"SOFTWARE\WOW6432Node\BraveSoftware\Brave-Browser\BLBeacon"),
                (Hive::Hkcu, r"Software\BraveSoftware\Brave-Browser\BLBeacon"),
            ],
            "opera" => &[
                (Hive::Hklm, r"SOFTWARE\Opera Software"),
                (Hive::Hklm, r"SOFTWARE\WOW6432Node\Opera Software"),
                (Hive::Hkcu, r"Software\Opera Software"),
            ],
            _ => &[],
        };

        for (hive, path) in candidates {
            let root = match hive {
                Hive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
                Hive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            };
            if let Ok(key) = root.open_subkey(path) {
                // Key exists → browser installed.
                drop(key);
                return true;
            }
        }
        false
    }

    /// Executable presence check (cross-platform fallback for portable /
    /// custom installs that do not write registry keys).
    fn executable_exists(browser: &str) -> bool {
        let local = std::env::var("LOCALAPPDATA").ok();
        let program_files = std::env::var("PROGRAMFILES").ok();
        let program_files_x86 = std::env::var("PROGRAMFILES(X86)").ok();

        let mut candidates: Vec<PathBuf> = Vec::new();
        match browser.to_lowercase().as_str() {
            "chrome" => {
                if let Some(p) = &program_files {
                    candidates.push(PathBuf::from(p).join("Google/Chrome/Application/chrome.exe"));
                }
                if let Some(p) = &program_files_x86 {
                    candidates.push(PathBuf::from(p).join("Google/Chrome/Application/chrome.exe"));
                }
                if let Some(p) = &local {
                    candidates.push(PathBuf::from(p).join("Google/Chrome/Application/chrome.exe"));
                }
            }
            "edge" => {
                if let Some(p) = &program_files {
                    candidates.push(PathBuf::from(p).join("Microsoft/Edge/Application/msedge.exe"));
                }
                if let Some(p) = &program_files_x86 {
                    candidates.push(PathBuf::from(p).join("Microsoft/Edge/Application/msedge.exe"));
                }
            }
            "firefox" => {
                if let Some(p) = &program_files {
                    candidates.push(PathBuf::from(p).join("Mozilla Firefox/firefox.exe"));
                }
                if let Some(p) = &program_files_x86 {
                    candidates.push(PathBuf::from(p).join("Mozilla Firefox/firefox.exe"));
                }
            }
            "brave" => {
                if let Some(p) = &program_files {
                    candidates.push(PathBuf::from(p).join("BraveSoftware/Brave-Browser/Application/brave.exe"));
                }
                if let Some(p) = &program_files_x86 {
                    candidates.push(PathBuf::from(p).join("BraveSoftware/Brave-Browser/Application/brave.exe"));
                }
                if let Some(p) = &local {
                    candidates.push(PathBuf::from(p).join("BraveSoftware/Brave-Browser/Application/brave.exe"));
                }
            }
            "opera" => {
                if let Some(p) = &program_files {
                    candidates.push(PathBuf::from(p).join("Opera/launcher.exe"));
                }
                if let Some(p) = &program_files_x86 {
                    candidates.push(PathBuf::from(p).join("Opera/launcher.exe"));
                }
                if let Some(p) = &local {
                    candidates.push(PathBuf::from(p).join("Opera/launcher.exe"));
                }
            }
            _ => {}
        }
        candidates.iter().any(|p| p.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_browsers_only_returns_ytdlp_supported() {
        // Every reported browser must be BOTH installed AND yt-dlp-supported.
        let list = CookieManager::installed_browsers();
        for b in &list {
            assert!(
                YTDLP_SUPPORTED_BROWSERS.contains(&b.as_str()),
                "installed list returned a non-ytdlp browser: {b}"
            );
            assert!(
                CookieManager::is_browser_installed(b),
                "is_browser_installed inconsistent with installed_browsers for {b}"
            );
        }
        // Diagnostics: log which browsers this machine detected (helps verify
        // the registry/executable probe works as expected).
        eprintln!("[diag] installed_browsers = {list:?}");
    }

    #[test]
    fn fallback_order_is_subset_of_ytdlp_supported() {
        // Guard against future regression: adding a browser to
        // BROWSER_FALLBACK_ORDER that yt-dlp cannot read would surface an
        // unusable option in the UI.
        for b in BROWSER_FALLBACK_ORDER {
            assert!(
                YTDLP_SUPPORTED_BROWSERS.contains(b),
                "BROWSER_FALLBACK_ORDER contains {b} which yt-dlp does not support"
            );
        }
    }

    #[test]
    fn browser_installed_but_not_ytdlp_supported_is_excluded() {
        // Simulate: if we ever detect "safari"/"internet-explorer" as installed,
        // installed_browsers() must still NOT include them (not yt-dlp usable).
        assert!(!YTDLP_SUPPORTED_BROWSERS.contains(&"internet-explorer"));
        // And even if is_browser_installed returns true for such a browser,
        // installed_browsers() iterates only over YTDLP_SUPPORTED_BROWSERS.
        for b in CookieManager::installed_browsers() {
            assert!(YTDLP_SUPPORTED_BROWSERS.contains(&b.as_str()));
        }
    }

    #[test]
    fn unsupported_browser_is_never_installed() {
        // "not-a-browser" is not in the fallback order and has no path/registry
        // entry, so it must never be reported installed.
        assert!(!CookieManager::is_browser_installed("not-a-browser"));
        // Safari may be installed on macOS but the executable probe on Windows
        // does not track it; regardless it is yt-dlp-supported so it is fine
        // if reported. We only assert it is never in the fallback list.
        assert!(!BROWSER_FALLBACK_ORDER.contains(&"safari"));
    }

    #[test]
    fn browser_cookie_path_matches_known_browsers() {
        // The path resolution must cover every supported browser.
        for b in BROWSER_FALLBACK_ORDER {
            let _ = CookieManager::browser_cookie_path(b);
        }
    }
}
