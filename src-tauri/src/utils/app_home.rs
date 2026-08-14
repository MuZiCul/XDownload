use std::path::{Path, PathBuf};

/// Resolves the application root directory:
/// - dev (cargo run / tauri dev): project root (where bin/ config/ downloads/ live)
/// - installed (packaged): the install directory (exe 所在目录)
pub struct AppHome;

impl AppHome {
    /// Application root directory — the install directory for packaged builds,
    /// or the project root (parent of src-tauri/) when running from the repo
    /// (dev mode, keeping bin/ outside Tauri's watch scope).
    pub fn root() -> PathBuf {
        // From the exe location, walk up to find the project root. Only a
        // dev build (exe under src-tauri/target/) can hit this; an installed
        // build never contains src-tauri/tauri.conf.json, so the exe's own
        // directory (the install dir) is returned instead.
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
            let mut current = Some(exe_dir);
            while let Some(dir) = current {
                if dir.join("src-tauri").join("tauri.conf.json").exists() {
                    return dir.to_path_buf();
                }
                current = dir.parent();
            }
            // Packaged build: exe lives in the install directory.
            return exe_dir.to_path_buf();
        }

        // Very unlikely fallback (current_exe failed).
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// bin/ directory
    pub fn bin_dir() -> PathBuf {
        Self::root().join("bin")
    }

    /// config/ directory
    pub fn config_dir() -> PathBuf {
        Self::root().join("config")
    }

    /// downloads/ directory
    pub fn downloads_dir() -> PathBuf {
        Self::root().join("downloads")
    }

    /// logs/ directory — daily log files live here (kept out of config/ so
    /// gitignoring the whole logs/ folder is trivial).
    pub fn logs_dir() -> PathBuf {
        Self::root().join("logs")
    }

    /// Temporary staging area where in-progress downloads are written before
    /// being moved to the real download directory on success. Because files
    /// only appear in the final folder after a successful (atomic) finish,
    /// an interrupted download leaves at most a partial file in the cache.
    pub fn download_cache_dir() -> PathBuf {
        Self::root().join("download_cache")
    }

    /// Ensure config/ directory exists
    pub fn ensure_config_dir() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::config_dir())
    }

    /// Ensure downloads/ directory exists
    pub fn ensure_downloads_dir() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::downloads_dir())
    }

    /// Ensure logs/ directory exists
    pub fn ensure_logs_dir() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::logs_dir())
    }
}
