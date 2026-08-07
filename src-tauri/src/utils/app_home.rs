use std::path::{Path, PathBuf};

/// Resolves the application root directory across different run modes:
/// - IDE / cargo run: project root (where bin/ config/ downloads/ live)
/// - jpackage / bundled: the install directory
pub struct AppHome;

impl AppHome {
    /// Application root directory — always the project root (parent of src-tauri/),
    /// NOT src-tauri/ itself. This keeps bin/ outside Tauri's watch scope
    /// so tool downloads never trigger hot-reload during dev.
    pub fn root() -> PathBuf {
        // Check for XDOWNLOAD_ROOT environment variable (set by launcher scripts)
        if let Ok(root) = std::env::var("XDOWNLOAD_ROOT") {
            let p = PathBuf::from(root);
            if p.exists() {
                return p;
            }
        }

        // Try to detect from executable location
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(Path::new("."));

            // jpackage: exe is at root, jars in app/
            if exe_dir.join("app").exists() || exe_dir.join("runtime").exists() {
                return exe_dir.to_path_buf();
            }

            // Walk up from exe dir to find project root
            // Project root = contains src-tauri/tauri.conf.json
            let mut current = Some(exe_dir);
            while let Some(dir) = current {
                if dir.join("src-tauri").join("tauri.conf.json").exists() {
                    return dir.to_path_buf();
                }
                current = dir.parent();
            }
        }

        // Fallback: walk up from CWD to find project root
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut current = Some(cwd.as_path());
        while let Some(dir) = current {
            if dir.join("src-tauri").join("tauri.conf.json").exists() {
                return dir.to_path_buf();
            }
            current = dir.parent();
        }

        cwd
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

    /// Ensure download_cache/ directory exists
    pub fn ensure_download_cache_dir() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::download_cache_dir())
    }
}
