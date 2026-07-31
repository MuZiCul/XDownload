use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};
use std::process::Stdio;

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

/// Find ffmpeg executable path
pub fn find_ffmpeg() -> PathBuf {
    let bin_dir = super::app_home::AppHome::bin_dir();

    #[cfg(windows)]
    let local_exe = bin_dir.join("ffmpeg.exe");
    #[cfg(not(windows))]
    let local_exe = bin_dir.join("ffmpeg");

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
    execute_inner(args, None::<LineCallback>, None::<LineCallback>, None, true).await
}

/// Execute a command with a timeout in seconds
pub async fn execute_with_timeout(args: &[&str], timeout_secs: u64) -> Result<CommandResult> {
    execute_inner(args, None::<LineCallback>, None::<LineCallback>, Some(timeout_secs), true).await
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
    execute_inner(args, stdout_cb, stderr_cb, timeout_secs, capture_stdout).await
}

async fn execute_inner(
    args: &[&str],
    stdout_cb: Option<LineCallback>,
    stderr_cb: Option<LineCallback>,
    timeout_secs: Option<u64>,
    capture_stdout: bool,
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

    // Don't show console window on Windows
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()
        .with_context(|| format!("failed to spawn: {:?}", args))?;

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
    let exit_code = if let Some(secs) = timeout_secs {
        match timeout(Duration::from_secs(secs), child.wait()).await {
            Ok(Ok(status)) => status.code().unwrap_or(-1),
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                // Timed out, kill the process
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(anyhow::anyhow!("command timed out after {}s", secs));
            }
        }
    } else {
        let status = child.wait().await?;
        status.code().unwrap_or(-1)
    };

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
