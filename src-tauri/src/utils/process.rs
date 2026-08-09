use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

/// Callback type used for process output lines
type LineCallback = Box<dyn Fn(String) + Send + 'static>;

/// Result of running an external command
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl CommandResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn stdout_text(&self) -> String {
        self.stdout.join("\n")
    }

    pub fn stderr_text(&self) -> String {
        self.stderr.join("\n")
    }
}

/// Registry of live child process PIDs, used to clean up running
/// downloads (yt-dlp / ffmpeg) when the app exits.
static RUNNING_CHILDREN: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

fn running_children() -> &'static Mutex<HashSet<u32>> {
    RUNNING_CHILDREN.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Register a spawned child process so it can be killed on quit.
pub fn register_child_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        if pid > 0 {
            if let Ok(mut set) = running_children().lock() {
                set.insert(pid);
            }
        }
    }
}

/// Remove a finished child process from the registry.
pub fn unregister_child_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        if let Ok(mut set) = running_children().lock() {
            set.remove(&pid);
        }
    }
}

/// Kill a single process and its whole process tree (e.g. yt-dlp + spawned
/// ffmpeg merge). On Windows uses `taskkill /T /F` which also kills children.
pub fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

/// Kill every registered child process (process tree on Windows).
pub fn kill_all_children() {
    let pids: Vec<u32> = running_children()
        .lock()
        .map(|s| s.iter().copied().collect())
        .unwrap_or_default();
    for pid in pids {
        kill_process_tree(pid);
    }
    let _ = running_children().lock().map(|mut s| s.clear());
}

/// Find yt-dlp executable path
pub fn find_ytdlp() -> PathBuf {
    let bin_dir = super::app_home::AppHome::bin_dir();

    // 1. Check bin/ directory (Windows: .exe, Unix: yt-dlp)
    #[cfg(windows)]
    let local_exe = bin_dir.join("yt-dlp.exe");
    #[cfg(not(windows))]
    let local_exe = bin_dir.join("yt-dlp");

    if local_exe.exists() {
        return local_exe;
    }

    // 2. Check system PATH
    #[cfg(windows)]
    let names: &[&str] = &["yt-dlp.exe", "yt-dlp"];
    #[cfg(not(windows))]
    let names: &[&str] = &["yt-dlp"];

    for name in names {
        if let Some(path) = std::env::split_paths(&std::env::var("PATH").unwrap_or_default())
            .map(|d| d.join(name))
            .find(|p| p.exists())
        {
            return path;
        }
    }

    // 3. Default: yt-dlp (let the system find it)
    PathBuf::from("yt-dlp")
}

/// Path of the ffmpeg binary bundled inside the app's `bin/` directory
/// (not from PATH). This is the only ffmpeg the downloader is allowed to use.
pub fn bundled_ffmpeg_path() -> PathBuf {
    let bin_dir = super::app_home::AppHome::bin_dir();
    #[cfg(windows)]
    {
        bin_dir.join("ffmpeg.exe")
    }
    #[cfg(not(windows))]
    {
        bin_dir.join("ffmpeg")
    }
}

/// Find ffmpeg executable path (bundled `bin/` first, then PATH).
pub fn find_ffmpeg() -> PathBuf {
    let local_exe = bundled_ffmpeg_path();
    if local_exe.exists() {
        return local_exe;
    }

    #[cfg(windows)]
    let names = &["ffmpeg.exe", "ffmpeg"];
    #[cfg(not(windows))]
    let names = &["ffmpeg"];

    for name in names {
        if let Some(path) = std::env::split_paths(&std::env::var("PATH").unwrap_or_default())
            .map(|d| d.join(name))
            .find(|p| p.exists())
        {
            return path;
        }
    }

    PathBuf::from("ffmpeg")
}

/// Check if yt-dlp is available (runs --version)
pub async fn is_ytdlp_available() -> bool {
    let ytdlp = find_ytdlp();
    match execute_with_timeout(&[ytdlp.to_str().unwrap_or("yt-dlp"), "--version"], 10).await {
        Ok(result) => result.is_success(),
        Err(_) => false,
    }
}

/// Check if ffmpeg is available (quick file existence check, then --version)
pub async fn is_ffmpeg_available() -> bool {
    let ffmpeg_path = find_ffmpeg();
    if ffmpeg_path.exists() && ffmpeg_path != PathBuf::from("ffmpeg") {
        return true;
    }
    match execute_with_timeout(&[ffmpeg_path.to_str().unwrap_or("ffmpeg"), "-version"], 10).await {
        Ok(result) => result.is_success(),
        Err(_) => false,
    }
}

/// Execute a command with no timeout
pub async fn execute(args: &[&str]) -> Result<CommandResult> {
    execute_inner(args, None::<LineCallback>, None::<LineCallback>, None, true, None).await
}

/// Execute a command with a timeout in seconds
pub async fn execute_with_timeout(args: &[&str], timeout_secs: u64) -> Result<CommandResult> {
    execute_inner(args, None::<LineCallback>, None::<LineCallback>, Some(timeout_secs), true, None).await
}

/// Execute a command with optional stdout/stderr callbacks and timeout.
/// Set `capture_stdout` to false to null stdout (avoids GBK pipe errors
/// on Windows when the child process writes non-UTF-8 to stdout).
pub async fn execute_with_callbacks(
    args: &[&str],
    stdout_cb: Option<LineCallback>,
    stderr_cb: Option<LineCallback>,
    timeout_secs: Option<u64>,
    capture_stdout: bool,
) -> Result<CommandResult> {
    execute_inner(args, stdout_cb, stderr_cb, timeout_secs, capture_stdout, None).await
}

/// Same as [`execute_with_callbacks`], but invokes `on_spawned` right after the
/// child process is spawned so the caller can retain its PID (e.g. to kill it
/// on cancellation).
pub async fn execute_with_callbacks_pid(
    args: &[&str],
    stdout_cb: Option<LineCallback>,
    stderr_cb: Option<LineCallback>,
    timeout_secs: Option<u64>,
    capture_stdout: bool,
    on_spawned: impl FnOnce(u32) + Send + 'static,
) -> Result<CommandResult> {
    execute_inner(args, stdout_cb, stderr_cb, timeout_secs, capture_stdout, Some(Box::new(on_spawned))).await
}

async fn execute_inner(
    args: &[&str],
    stdout_cb: Option<LineCallback>,
    stderr_cb: Option<LineCallback>,
    timeout_secs: Option<u64>,
    capture_stdout: bool,
    on_spawned: Option<Box<dyn FnOnce(u32) + Send + 'static>>,
) -> Result<CommandResult> {
    if args.is_empty() {
        return Err(anyhow::anyhow!("empty command"));
    }

    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    if capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null()); // avoids GBK pipe errors with yt-dlp on Windows
    }
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    // yt-dlp (Python) on Chinese Windows defaults to GBK encoding for
    // stdout.  Piped stdout has no console, so writes fail with
    // "Invalid argument".  PYTHONUTF8=1 forces Python ≥3.7 to use
    // UTF-8 mode on Windows (more reliable than PYTHONIOENCODING).
    cmd.env("PYTHONUTF8", "1");
    // Belt-and-braces: force the child's stdin/stdout/stderr to UTF-8 so
    // piped stdout does not hit GBK "Invalid argument" errors on Chinese
    // Windows (the built yt-dlp.exe may ignore PYTHONUTF8 alone).
    cmd.env("PYTHONIOENCODING", "utf-8");

    // Don't show console window on Windows
    // (tokio::process::Command has its own `creation_flags` on Windows)
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()
        .with_context(|| format!("failed to spawn: {:?}", args))?;

    let child_pid = child.id();
    register_child_pid(child_pid);
    if let Some(cb) = on_spawned {
        if let Some(pid) = child_pid {
            cb(pid);
        }
    }

    let stdout_opt = child.stdout.take();
    let stderr = child.stderr.take()
        .ok_or_else(|| anyhow::anyhow!("no stderr"))?;

    // Read stdout and stderr concurrently
    let stdout_handle: tokio::task::JoinHandle<Vec<String>> = {
        let stdout_cb = stdout_cb;
        tokio::spawn(async move {
            let mut lines = Vec::new();
            if let Some(stdout) = stdout_opt {
                use tokio::io::BufReader;
                let mut reader = BufReader::new(stdout);
                let mut buf = String::new();
                loop {
                    buf.clear();
                    match reader.read_line(&mut buf).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let line = buf.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
                            if let Some(ref cb) = stdout_cb {
                                cb(line.clone());
                            }
                            lines.push(line);
                        }
                        Err(_) => break,
                    }
                }
            }
            lines
        })
    };

    let stderr_handle = {
        let stderr_cb = stderr_cb;
        tokio::spawn(async move {
            let mut lines = Vec::new();
            use tokio::io::BufReader;
            let mut reader = BufReader::new(stderr);
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let line = buf.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
                        if let Some(ref cb) = stderr_cb {
                            cb(line.clone());
                        }
                        lines.push(line);
                    }
                    Err(_) => break,
                }
            }
            lines
        })
    };

    // Wait for process with optional timeout
    let wait_result = if let Some(secs) = timeout_secs {
        match timeout(Duration::from_secs(secs), child.wait()).await {
            Ok(Ok(status)) => Ok(status.code().unwrap_or(-1)),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timed out, kill the process
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err(std::io::Error::other(anyhow::anyhow!("command timed out after {}s", secs)))
            }
        }
    } else {
        child.wait().await.map(|s| s.code().unwrap_or(-1))
    };

    // Remove from live-child registry regardless of outcome
    unregister_child_pid(child_pid);

    let exit_code = wait_result?;

    let stdout_lines = stdout_handle.await.unwrap_or_else(|_| Vec::new());
    let stderr_lines = stderr_handle.await.unwrap_or_default();

    Ok(CommandResult {
        exit_code,
        stdout: stdout_lines,
        stderr: stderr_lines,
    })
}

/// Cancel a running yt-dlp process by killing its child
pub async fn kill_process(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

/// Check if we're running on Windows
pub fn is_windows() -> bool {
    cfg!(windows)
}
