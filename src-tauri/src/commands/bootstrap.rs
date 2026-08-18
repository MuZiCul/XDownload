use tauri::{AppHandle, Emitter, Manager};

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

/// Get the application root directory path
#[tauri::command]
pub fn get_root_dir() -> String {
    crate::utils::app_home::AppHome::root()
        .to_string_lossy()
        .to_string()
}

/// Open the application root directory in the system file manager
#[tauri::command]
pub fn open_root_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let root = crate::utils::app_home::AppHome::root();
    app.opener()
        .open_path(root.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("failed to open root dir: {}", e))
}

/// Open the download directory in the system file manager.
/// Uses the configured absolute `download_dir` when present, otherwise the
/// default `downloads/` folder. Creates the directory if it does not exist.
#[tauri::command]
pub fn open_download_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let dir = match crate::services::config::ConfigManager::load_download_dir() {
        Some(d) if !d.is_empty() && std::path::Path::new(&d).is_absolute() => {
            std::path::PathBuf::from(d)
        }
        _ => crate::utils::app_home::AppHome::downloads_dir(),
    };

    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(format!("failed to create download dir: {}", e));
    }

    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("failed to open download dir: {}", e))
}

/// Open the file manager at the location of a downloaded video file.
///
/// When download history knows the file and it still exists on disk, the file
/// is located/selected in the system file manager (Windows: `explorer /select`).
/// Otherwise it falls back to opening the download directory.
#[tauri::command]
pub fn open_download_path(app: tauri::AppHandle, video_id: String) -> Result<(), String> {
    let file_path = crate::services::download_history::DownloadHistory::get(&video_id)
        .and_then(|rec| rec.file_path)
        .filter(|p| !p.trim().is_empty())
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists());

    let Some(path) = file_path else {
        tracing::info!(
            "open_download_path: no existing file for '{}', opening download dir",
            video_id
        );
        return open_download_dir(app);
    };

    #[cfg(windows)]
    {
        // Use the plugin's official "reveal in file manager" API, which on
        // Windows calls SHOpenFolderAndSelectItems. This is immune to the
        // command-line quoting pitfalls of `explorer /select,<path>` — with a
        // space in the filename explorer mis-parses the argument and opens an
        // unrelated folder (e.g. Documents).
        use tauri_plugin_opener::OpenerExt;
        match app.opener().reveal_item_in_dir(&path) {
            Ok(_) => {
                tracing::info!(
                    "open_download_path: revealing '{}' in explorer",
                    path.display()
                );
                Ok(())
            }
            Err(e) => {
                // Reveal failed — fall back to opening the parent directory so
                // the user can still reach the file.
                tracing::warn!(
                    "open_download_path: reveal failed ({}), opening parent dir",
                    e
                );
                let dir = path.parent().unwrap_or(&path);
                app.opener()
                    .open_path(dir.to_string_lossy().to_string(), None::<&str>)
                    .map_err(|e| format!("failed to open file location: {}", e))
            }
        }
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_opener::OpenerExt;
        let dir = path.parent().unwrap_or(&path);
        app.opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| format!("failed to open file location: {}", e))
    }
}

/// Open the file manager with a specific file selected (reveal).
/// Used by the download-complete toast: clicking the title opens the location
/// of the finished file. The path comes directly from the `download-finished`
/// event payload (`file_path`), so no history lookup is needed.
#[tauri::command]
pub fn open_file_path(app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(file_path);
    if !path.exists() {
        tracing::warn!(
            "open_file_path: '{}' does not exist, opening download dir",
            path.display()
        );
        return open_download_dir(app);
    }

    #[cfg(windows)]
    {
        use tauri_plugin_opener::OpenerExt;
        match app.opener().reveal_item_in_dir(&path) {
            Ok(_) => {
                tracing::info!(
                    "open_file_path: revealing '{}' in explorer",
                    path.display()
                );
                Ok(())
            }
            Err(e) => {
                // Reveal failed — fall back to opening the parent directory so
                // the user can still reach the file.
                tracing::warn!(
                    "open_file_path: reveal failed ({}), opening parent dir",
                    e
                );
                let dir = path.parent().unwrap_or(&path);
                app.opener()
                    .open_path(dir.to_string_lossy().to_string(), None::<&str>)
                    .map_err(|e| format!("failed to open file location: {}", e))
            }
        }
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_opener::OpenerExt;
        let dir = path.parent().unwrap_or(&path);
        app.opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| format!("failed to open file location: {}", e))
    }
}

/// Clean up running child processes (including active downloads) and quit the
/// app. `save_progress`: when true, the multi-task queue is force-persisted to
/// `config/queue.json` first so unfinished tasks can resume on next launch.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle, save_progress: bool) {
    tracing::info!(
        "quit_app: cleaning up child processes and exiting (save_progress={})",
        save_progress
    );
    if save_progress {
        if let Some(state) = app.try_state::<crate::commands::download::DownloaderState>() {
            // 先持久化当前队列（running/queued/paused 状态原样保存，重启后
            // 按状态恢复），再标记「退出中」——此后被 kill 的 worker 会静默
            // 结束，不会给正在下载的任务留下虚假的失败历史记录。
            state.queue.save_now();
            state.queue.prepare_exit();
        }
    }
    // 用户主动退出：无论当前是否有任务，一律恢复系统正常休眠（退出后
    // 防休眠请求不再有意义；重启后由启动时的 sync_keep_awake 重新接管）。
    crate::utils::keep_awake::disable();
    crate::utils::process::kill_all_children();
    // Destroy the main window first so the WebView2 environment is torn down
    // cleanly — avoids the benign "Failed to unregister class ... 1412" error
    // that otherwise appears when calling exit() while the webview is alive.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.destroy();
    }
    app.exit(0);
}

/// Check if yt-dlp is available and get its version (single call)
#[tauri::command]
pub async fn check_ytdlp() -> serde_json::Value {
    let ytdlp = crate::utils::process::find_ytdlp();
    let ytdlp_str = ytdlp.to_str().unwrap_or("yt-dlp");

    // The PyInstaller-built yt-dlp.exe cold-starts slowly (self-extraction +
    // antivirus scan) and can exceed a short timeout, falsely reporting
    // "not installed". Retry once with a generous timeout.
    for attempt in 0..2 {
        let result =
            crate::utils::process::execute_with_timeout(&[ytdlp_str, "--version"], 15).await;
        if let Ok(result) = result {
            if result.is_success() && !result.stdout.is_empty() {
                let version = result.stdout[0].trim().to_string();
                return serde_json::json!({ "available": true, "version": version });
            }
        }
        if attempt == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    // Fallback: the binary exists but we could not read its version (e.g.
    // transient slowness). Report it as available to avoid a false
    // "not installed" state in the status bar.
    if ytdlp.exists() {
        return serde_json::json!({ "available": true, "version": Option::<String>::None });
    }

    serde_json::json!({ "available": false, "version": Option::<String>::None })
}

/// Whether the bundled ffmpeg exists on disk under `bin/` (not from PATH).
/// Downloads are rejected up-front unless this is true.
#[tauri::command]
pub fn is_ffmpeg_bundled() -> bool {
    crate::utils::process::bundled_ffmpeg_path().exists()
}

/// Check if ffmpeg is available and get its version
#[tauri::command]
pub async fn check_ffmpeg() -> serde_json::Value {
    let ffmpeg = crate::utils::process::find_ffmpeg();
    if !ffmpeg.exists() {
        return serde_json::json!({ "available": false, "version": Option::<String>::None });
    }

    let ffmpeg_str = ffmpeg.to_str().unwrap_or("ffmpeg");
    for attempt in 0..2 {
        let result =
            crate::utils::process::execute_with_timeout(&[ffmpeg_str, "-version"], 10).await;
        if let Ok(result) = result {
            if result.is_success() && !result.stdout.is_empty() {
                // Parse "ffmpeg version 7.1-essentials_build-..." → "7.1"
                let version = crate::commands::update::parse_ffmpeg_version_export(&result.stdout[0]);
                return serde_json::json!({ "available": true, "version": version });
            }
        }
        if attempt == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    serde_json::json!({ "available": false, "version": Option::<String>::None })
}

/// Download yt-dlp with progress events
#[tauri::command]
pub async fn download_ytdlp(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();
    let app_mode = app.clone();
    // 设置页「代理下载」开关开启 → 强制走代理；否则自动（直连优先，失败切代理）。
    let force_mode = if crate::services::config::ConfigManager::load()
        .tools_use_proxy
        .unwrap_or(false)
    {
        Some("proxy")
    } else {
        None
    };
    let result = crate::services::bootstrap::Bootstrap::download_ytdlp(
        move |pct| {
            let _ = app_clone.emit("bootstrap-progress", serde_json::json!({
                "tool": "yt-dlp",
                "percent": pct,
            }));
        },
        &move |mode: &str| {
            let _ = app_mode.emit("bootstrap-progress", serde_json::json!({
                "tool": "yt-dlp",
                "mode": mode,
            }));
        },
        force_mode,
    )
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
    let app_mode = app.clone();
    // 设置页「代理下载」开关开启 → 强制走代理；否则自动（直连优先，失败切代理）。
    let force_mode = if crate::services::config::ConfigManager::load()
        .tools_use_proxy
        .unwrap_or(false)
    {
        Some("proxy")
    } else {
        None
    };
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
        &move |mode: &str| {
            let _ = app_mode.emit("bootstrap-progress", serde_json::json!({
                "tool": "ffmpeg",
                "mode": mode,
            }));
        },
        force_mode,
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

/// Result of looking up the XDownload uninstall entry in the Windows registry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UninstallInfo {
    pub installed: bool,
    pub uninstall_string: Option<String>,
    pub display_name: Option<String>,
}

/// Read the XDownload uninstall entry from the Windows registry.
///
/// Checks the four standard locations (HKLM/HKCU × 64-bit/32-bit views):
///   HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\XDownload
///   HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\XDownload
///   HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\XDownload
///   HKCU\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\XDownload
/// The 32-bit view is opened with KEY_WOW64_32KEY, which transparently maps to
/// the WOW6432Node paths on 64-bit Windows.
#[tauri::command]
pub fn get_uninstall_info() -> UninstallInfo {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        const BASE_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
        // (hive, label) — each checked under both 64-bit and 32-bit views.
        let hives: [(RegKey, &str); 2] = [
            (RegKey::predef(HKEY_LOCAL_MACHINE), "HKLM"),
            (RegKey::predef(HKEY_CURRENT_USER), "HKCU"),
        ];

        for (hive, hive_name) in &hives {
            for flags in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
                if let Ok(root) = hive.open_subkey_with_flags(BASE_PATH, flags) {
                    if let Ok(key) = root.open_subkey("XDownload") {
                        let uninstall_string: Option<String> =
                            key.get_value("UninstallString").ok();
                        let display_name: Option<String> = key.get_value("DisplayName").ok();
                        tracing::info!(
                            "get_uninstall_info: found XDownload in {} (flags=0x{:x})",
                            hive_name,
                            flags
                        );
                        return UninstallInfo {
                            installed: true,
                            uninstall_string: uninstall_string.filter(|s| !s.trim().is_empty()),
                            display_name,
                        };
                    }
                }
            }
        }

        UninstallInfo {
            installed: false,
            uninstall_string: None,
            display_name: None,
        }
    }
    #[cfg(not(windows))]
    {
        UninstallInfo {
            installed: false,
            uninstall_string: None,
            display_name: None,
        }
    }
}

/// Split an UninstallString into (executable, args), respecting quotes.
/// e.g. `"C:\Program Files\XDownload\Uninstall.exe" /S` →
///      ("C:\Program Files\XDownload\Uninstall.exe", ["/S"])
fn parse_uninstall_command(uninstall_string: &str) -> (String, Vec<String>) {
    let s = uninstall_string.trim();
    if s.is_empty() {
        return (String::new(), Vec::new());
    }

    // Executable is quoted → extract until the closing quote.
    if let Some(stripped) = s.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            let exe = stripped[..end].to_string();
            let rest = stripped[end + 1..].trim();
            return (exe, parse_args(rest));
        }
    }

    // No leading quote: the executable is the first whitespace-delimited token.
    let mut parts = s.splitn(2, char::is_whitespace);
    let exe = parts.next().unwrap_or("").trim().to_string();
    let rest = parts.next().unwrap_or("").trim();
    (exe, parse_args(rest))
}

/// Tokenize an argument string, treating double quotes as grouping.
fn parse_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quote && chars.peek() == Some(&'"') {
                    current.push('"'); // escaped quote `""`
                    chars.next();
                } else {
                    in_quote = !in_quote;
                }
            }
            ' ' | '\t' if !in_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// NSIS uninstallers are typically named "Uninstall.exe" / "uninst.exe".
fn is_nsis_uninstaller(exe: &str) -> bool {
    let stem = std::path::Path::new(exe)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    stem.starts_with("uninstall") || stem.starts_with("uninst")
}

/// Best-effort recursive removal of a runtime directory (`bin/` or `config/`).
/// Missing directories are skipped; failures only log a warning so the
/// uninstall flow never aborts because of a locked file (e.g. the tracing
/// appender holding `config/xdownload.log.*` open on Windows).
fn cleanup_runtime_dir(dir: &std::path::Path, label: &str) {
    if !dir.exists() {
        tracing::debug!(
            "cleanup: {} dir not present, skipping ({})",
            label,
            dir.display()
        );
        return;
    }
    match std::fs::remove_dir_all(dir) {
        Ok(()) => tracing::info!("cleanup: removed {} dir ({})", label, dir.display()),
        Err(e) => tracing::warn!(
            "cleanup: failed to remove {} dir '{}': {}",
            label,
            dir.display(),
            e
        ),
    }
}

/// Best-effort removal of the `HKCU\Software\XDownload` registry key (holds the
/// disclaimer acceptance state). Failures only log a warning.
fn cleanup_registry() {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        const REG_KEY_PATH: &str = r"Software\XDownload";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.delete_subkey_all(REG_KEY_PATH) {
            Ok(()) => tracing::info!("cleanup: removed registry key HKCU\\{}", REG_KEY_PATH),
            Err(e) => tracing::warn!(
                "cleanup: failed to remove registry key HKCU\\{}: {}",
                REG_KEY_PATH,
                e
            ),
        }
    }
    #[cfg(not(windows))]
    {
        tracing::debug!("cleanup: registry cleanup skipped on non-Windows");
    }
}

/// Launch the registered uninstaller (if any), then clean up child processes
/// and exit the app. Returns `false` when no usable uninstall entry is found so
/// the caller can fall back to opening the system uninstall panel.
#[tauri::command]
pub fn uninstall_app(app: tauri::AppHandle) -> Result<bool, String> {
    let info = get_uninstall_info();
    let uninstall_string = match (info.installed, info.uninstall_string) {
        (true, Some(s)) if !s.trim().is_empty() => s,
        _ => {
            tracing::info!("uninstall_app: no uninstall entry found, handled=false");
            return Ok(false);
        }
    };

    let (exe, mut args) = parse_uninstall_command(&uninstall_string);
    if exe.is_empty() {
        tracing::info!("uninstall_app: empty uninstaller path, handled=false");
        return Ok(false);
    }

    // NSIS uninstallers support silent mode via /S — append if not already set.
    if is_nsis_uninstaller(&exe) && !args.iter().any(|a| a.eq_ignore_ascii_case("/S")) {
        args.push("/S".to_string());
    }

    // --- Pre-uninstall cleanup (best-effort; failures only warn, never abort) ---
    // Only bin/ and config/ runtime dirs are removed — downloads/ (user data)
    // is intentionally preserved. Never touches AppHome::root() itself.
    // 1. Kill running child processes (yt-dlp / ffmpeg) to release file locks.
    crate::utils::process::kill_all_children();
    // 2. Remove the runtime bin/ directory (downloaded yt-dlp / ffmpeg).
    cleanup_runtime_dir(&crate::utils::app_home::AppHome::bin_dir(), "bin");
    // 3. Remove the runtime config/ directory (settings.json, logs).
    cleanup_runtime_dir(&crate::utils::app_home::AppHome::config_dir(), "config");
    // 4. Remove the HKCU\Software\XDownload registry key (disclaimer state).
    cleanup_registry();

    // 5. Launch the uninstaller.
    std::process::Command::new(&exe)
        .args(&args)
        .spawn()
        .map_err(|e| format!("failed to launch uninstaller '{}': {}", exe, e))?;

    tracing::info!(
        "uninstall_app: launched uninstaller '{}' with args {:?}",
        exe,
        args
    );
    app.exit(0);
    Ok(true)
}

/// Open the Windows "Programs and Features" (Add/Remove Programs) panel.
#[tauri::command]
pub fn open_uninstall_panel() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "appwiz.cpl"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW — avoid console flash
            .spawn()
            .map_err(|e| format!("failed to open uninstall panel: {}", e))?;
        tracing::info!("open_uninstall_panel: launched appwiz.cpl");
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("open_uninstall_panel is only supported on Windows".to_string())
    }
}
