use crate::downloader::parser::parse_video_json;
use crate::downloader::progress::parse_progress_line;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use crate::services::proxy::ProxyConfig;
use crate::utils::process;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

/// Return a path that does not currently exist by appending " (n)" before the
/// extension: `name.ext` -> `name (1).ext`, `name (2).ext`, ...
fn unique_path(path: &std::path::Path) -> PathBuf {
    let parent = path.parent().unwrap_or(std::path::Path::new(""));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let (stem, ext) = match name.rfind('.') {
        Some(idx) if idx > 0 => (name[..idx].to_string(), Some(name[idx..].to_string())),
        _ => (name.to_string(), None),
    };
    let mut n = 1;
    loop {
        let candidate = match &ext {
            Some(e) => format!("{} ({}){}", stem, n, e),
            None => format!("{} ({})", stem, n),
        };
        let cand_path = parent.join(&candidate);
        if !cand_path.exists() {
            return cand_path;
        }
        n += 1;
    }
}

/// RAII guard that releases the download-in-progress flag when dropped, so the
/// mutual-exclusion lock is freed on success, error, and cancellation alike.
struct DownloadLock<'a>(&'a AtomicBool);

impl Drop for DownloadLock<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Core downloader wrapping yt-dlp CLI.
pub struct YtDlpDownloader {
    ytdlp_path: String,
    cookies_from_browser: Mutex<Option<String>>,
    cookies_file: Mutex<Option<String>>,
    cancel_flag: Arc<AtomicBool>,
    /// Whether a download task is currently running. Acts as a mutex so two
    /// yt-dlp processes can never run concurrently (the frontend may lose its
    /// local `downloading` state when switching tabs).
    is_downloading: Arc<AtomicBool>,
    /// PID of the currently running download child process (yt-dlp).
    /// Used to actually terminate the process tree on cancellation.
    current_pid: Arc<Mutex<Option<u32>>>,
}

impl YtDlpDownloader {
    pub fn new() -> Self {
        let ytdlp_path = process::find_ytdlp()
            .to_str()
            .unwrap_or("yt-dlp")
            .to_string();

        Self {
            ytdlp_path,
            cookies_from_browser: Mutex::new(None),
            cookies_file: Mutex::new(None),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            is_downloading: Arc::new(AtomicBool::new(false)),
            current_pid: Arc::new(Mutex::new(None)),
        }
    }

    /// Whether a download task is currently running.
    pub fn is_downloading(&self) -> bool {
        self.is_downloading.load(Ordering::SeqCst)
    }

    pub fn set_cookies_from_browser(&self, browser: &str) {
        let mut c = self.cookies_from_browser.lock().unwrap();
        *c = Some(browser.to_string());
        let mut f = self.cookies_file.lock().unwrap();
        *f = None;
    }

    pub fn set_cookies_file(&self, path: &str) {
        let mut f = self.cookies_file.lock().unwrap();
        *f = Some(path.to_string());
        let mut c = self.cookies_from_browser.lock().unwrap();
        *c = None;
    }

    pub fn get_cookies_from_browser(&self) -> Option<String> {
        self.cookies_from_browser.lock().unwrap().clone()
    }

    pub fn get_cookies_file(&self) -> Option<String> {
        self.cookies_file.lock().unwrap().clone()
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        // Terminate the running yt-dlp process (and its ffmpeg children) so
        // cancellation actually stops the download instead of just pausing the UI.
        if let Some(pid) = *self.current_pid.lock().unwrap() {
            tracing::info!("cancel: killing download process tree pid={}", pid);
            process::kill_process_tree(pid);
        }
    }

    fn reset_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    pub async fn fetch_video_info(&self, url: &str) -> Result<VideoInfo> {
        self.reset_cancel();

        let mut cmd = self.build_base_command();

        // Add cookies from downloader state
        let browser = self.cookies_from_browser.lock().unwrap().clone();
        let file = self.cookies_file.lock().unwrap().clone();
        if let Some(ref b) = browser {
            if !b.is_empty() {
                cmd.push("--cookies-from-browser".to_string());
                cmd.push(b.clone());
            }
        } else if let Some(ref f) = file {
            if !f.is_empty() {
                cmd.push("--cookies".to_string());
                cmd.push(f.clone());
            }
        }

        cmd.push("--dump-json".to_string());
        cmd.push("--no-playlist".to_string());
        cmd.push(url.to_string());

        let result = self.execute_with_cookies_retry(&cmd, 30).await?;

        if !result.is_success() {
            let stderr = result.stderr_text();
            if stderr.contains("age") || stderr.contains("login") || stderr.contains("unavailable") {
                anyhow::bail!("需要登录或年龄验证，请设置 Cookies:\n{}", stderr);
            }
            anyhow::bail!("yt-dlp 解析失败: {}", stderr);
        }

        let json = result.stdout_text();
        if json.trim().is_empty() {
            anyhow::bail!("无法获取视频信息，请检查 URL 是否正确");
        }

        // `--dump-json` may emit several JSON lines (e.g. a tweet containing
        // multiple media entries) plus non-JSON log lines. Parse EVERY valid
        // JSON line so multi-media tweets report all entries; the first entry
        // becomes the main info and `media_count` records the total.
        let mut parsed: Option<VideoInfo> = None;
        let mut media_count = 0usize;
        for line in json.lines() {
            let line = line.trim();
            if !line.starts_with('{') {
                continue; // skip log / warning lines
            }
            if let Ok(info) = parse_video_json(line) {
                if parsed.is_none() {
                    parsed = Some(info);
                }
                media_count += 1;
            }
        }

        match parsed {
            Some(mut info) => {
                // Always report at least one media entry, and keep the original
                // input URL so the download step targets the whole tweet
                // (i.e. all of its media entries).
                info.media_count = media_count.max(1);
                info.url = url.to_string();
                Ok(info)
            }
            None => {
                let preview: String = json.chars().take(300).collect();
                anyhow::bail!(
                    "无法解析视频信息，请检查 URL 是否正确:\n{}",
                    preview
                )
            }
        }
    }

    /// Download a video. Returns the final file path (from
    /// `--print-to-file after_move:filepath`) on success, `None` if the
    /// process failed without producing stderr output.
    pub async fn download(
        &self,
        config: &DownloadConfig,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<Option<String>> {
        // Mutual exclusion: reject concurrent downloads. The flag is released
        // automatically by `DownloadLock` when this method returns.
        if self.is_downloading.swap(true, Ordering::SeqCst) {
            anyhow::bail!("已有下载任务正在进行，请等待当前任务完成");
        }
        let _lock = DownloadLock(&self.is_downloading);

        self.reset_cancel();

        let mut cmd = self.build_base_command();
        cmd.push("-f".to_string());
        cmd.push(config.format_id.clone());
        cmd.push("-o".to_string());
        cmd.push(config.output_path());
        cmd.push("--socket-timeout".to_string());
        cmd.push(config.socket_timeout.to_string());
        // Deliberately NOT passing --no-playlist here: a tweet with several
        // media entries is exposed by yt-dlp as multiple playlist items, and
        // --no-playlist would silently download only the first one. An
        // explicit playlist_items narrows the download; otherwise all media
        // entries of the tweet are downloaded.
        if let Some(ref items) = config.playlist_items {
            if !items.is_empty() {
                cmd.push("--playlist-items".to_string());
                cmd.push(items.clone());
            }
        }

        std::fs::create_dir_all(&config.output_dir).ok();

        if let Some(ref archive) = config.download_archive {
            if !archive.is_empty() {
                cmd.push("--download-archive".to_string());
                cmd.push(archive.clone());
            }
        }

        if config.extract_audio {
            cmd.push("-x".to_string());
            cmd.push("--audio-format".to_string());
            cmd.push("mp3".to_string());
            cmd.push("--audio-quality".to_string());
            cmd.push("0".to_string());
        }

        if config.embed_subtitles {
            cmd.push("--embed-subs".to_string());
            cmd.push("--write-auto-subs".to_string());
        }

        if config.embed_thumbnail {
            cmd.push("--embed-thumbnail".to_string());
        }
        if config.write_thumbnail {
            cmd.push("--write-thumbnail".to_string());
        }

        if let Some(ref proxy) = config.proxy {
            if !proxy.is_empty() {
                cmd.push("--proxy".to_string());
                cmd.push(proxy.clone());
            }
        }

        // Cookies: prefer per-request config, fall back to downloader state
        let (cookies_browser, cookies_file) =
            if config.cookies_from_browser.is_some() || config.cookies_file.is_some() {
                (config.cookies_from_browser.clone(), config.cookies_file.clone())
            } else {
                let browser = self.cookies_from_browser.lock().unwrap().clone();
                let file = self.cookies_file.lock().unwrap().clone();
                (browser, file)
            };

        if let Some(ref b) = cookies_browser {
            if !b.is_empty() {
                cmd.push("--cookies-from-browser".to_string());
                cmd.push(b.clone());
            }
        } else if let Some(ref f) = cookies_file {
            if !f.is_empty() {
                cmd.push("--cookies".to_string());
                cmd.push(f.clone());
            }
        }

        if config.max_height > 0 {
            cmd.push("--format-sort".to_string());
            cmd.push(format!("+height:{}", config.max_height));
        }

        // Default progress goes to stderr (no --progress-template which
        // would redirect to stdout and trigger GBK pipe errors on Windows).
        cmd.push("--newline".to_string());
        cmd.push("--progress".to_string());
        // Force a periodic, machine-readable progress line. Without a
        // --progress-template, HLS/native downloads only emit a handful of
        // progress lines (0% → 100%), so the UI would jump straight to 100%.
        // The pipe-delimited format is parsed by parse_progress_line.
        cmd.push("--progress-template".to_string());
        cmd.push(
            "download:%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._percent_str)s|%(progress.status)s"
                .to_string(),
        );

        // Record the final file path (after all post-processing) to a temp
        // file so we can remember where the video was saved.
        let tmp_path = std::env::temp_dir().join("xdownload_last_path.txt");
        let _ = std::fs::remove_file(&tmp_path);
        cmd.push("--print-to-file".to_string());
        cmd.push("after_move:filepath".to_string());
        cmd.push(tmp_path.to_string_lossy().to_string());

        cmd.push(config.url.clone());

        // Diagnostic: log the full command so the actually-selected format can
        // be confirmed from the log file (used to debug resolution mismatches).
        tracing::info!("yt-dlp download command: {}", cmd.join(" "));

        let cancel = self.cancel_flag.clone();
        let args_refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();

        // Parse progress from BOTH pipes: depending on the downloader and
        // options, yt-dlp may write "[download] xx%" to stdout or stderr.
        // The callback is shared behind a Mutex; a counter tells us how many
        // progress lines were actually parsed (logged after the download).
        let progress_cb = Arc::new(Mutex::new(progress_cb));
        let stdout_progress = progress_cb.clone();
        let stderr_progress = progress_cb.clone();
        let progress_count = Arc::new(AtomicUsize::new(0));
        let progress_count_stdout = progress_count.clone();
        let progress_count_stderr = progress_count.clone();
        let cancel_stderr = cancel.clone();
        let current_pid = self.current_pid.clone();
        let result = process::execute_with_callbacks_pid(
            &args_refs,
            // stdout → informational lines + possible progress lines
            Some(Box::new(move |line: String| {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(progress) = parse_progress_line(&line) {
                    if let Ok(guard) = stdout_progress.lock() {
                        guard(progress);
                    }
                    progress_count_stdout.fetch_add(1, Ordering::SeqCst);
                }
                // Log key format-selection / merge lines for diagnosis, but
                // skip pure progress lines like "[download]  45.2%".
                let is_pure_progress = line.starts_with("[download]") && line.contains('%');
                if !is_pure_progress
                    && (line.contains("[info]")
                        || line.contains("[Merger]")
                        || line.contains("[ExtractAudio]")
                        || line.contains("ERROR")
                        || line.contains("WARNING"))
                {
                    tracing::info!("yt-dlp: {}", line);
                }
            })),
            // stderr → yt-dlp progress lines ("[download] xx% ...") + errors
            Some(Box::new(move |line: String| {
                if cancel_stderr.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(progress) = parse_progress_line(&line) {
                    if let Ok(guard) = stderr_progress.lock() {
                        guard(progress);
                    }
                    progress_count_stderr.fetch_add(1, Ordering::SeqCst);
                }
                if line.contains("ERROR") || line.contains("error") {
                    tracing::error!("{}", line);
                }
            })),
            None,
            true, // capture_stdout — informational lines + possible progress
            move |pid| {
                if let Ok(mut guard) = current_pid.lock() {
                    *guard = Some(pid);
                }
            },
        )
        .await?;

        tracing::info!(
            "download progress lines parsed: {}",
            progress_count.load(Ordering::SeqCst)
        );

        // Download finished (or was cancelled) — clear the tracked PID.
        if let Ok(mut guard) = self.current_pid.lock() {
            *guard = None;
        }

        if !result.is_success() {
            let stderr = result.stderr_text();
            if !stderr.is_empty() {
                anyhow::bail!("下载失败: {}", stderr);
            }
            return Ok(None);
        }

        // Read the actual saved path written by --print-to-file. When multiple
        // media entries are downloaded (multi-media tweets) the file contains
        // one path per line — keep the LAST one for the history record.
        let saved_path = std::fs::read_to_string(&tmp_path)
            .ok()
            .map(|s| {
                s.lines()
                    .last()
                    .map(|l| l.trim().to_string())
                    .unwrap_or_default()
            })
            .filter(|s| !s.is_empty());

        // Clean the filename (keep only Chinese / letters / digits / - # +)
        // so the saved file (and the history record) has a clean name. When the
        // cleaned target already exists (e.g. a previous download), the freshly
        // downloaded file is kept under a numbered name instead of being
        // discarded — otherwise re-downloads would silently keep the old file
        // while the history still records a success.
        let saved_path = saved_path.map(|p| {
            let new_path =
                crate::services::download_history::DownloadHistory::sanitize_filename(&p);
            if new_path == p {
                return p;
            }
            let src = std::path::Path::new(&p);
            let dst = std::path::Path::new(&new_path);
            if dst.exists() {
                // A cleaned-name file already exists — keep the new download
                // under a numbered name (name (1).ext, name (2).ext, …).
                let unique = unique_path(dst);
                tracing::info!(
                    "cleaned target exists, renaming {} -> {}",
                    src.display(),
                    unique.display()
                );
                let _ = std::fs::rename(src, &unique);
                unique.to_string_lossy().to_string()
            } else {
                tracing::info!("renaming {} -> {}", src.display(), dst.display());
                let _ = std::fs::rename(src, dst);
                new_path
            }
        });

        Ok(saved_path)
    }

    fn build_base_command(&self) -> Vec<String> {
        let mut cmd = Vec::new();
        cmd.push(self.ytdlp_path.clone());
        cmd.push("--no-warnings".to_string());
        cmd.push("--no-color".to_string());
        // Force UTF-8 output. Without this, yt-dlp (Python) writes stdout in
        // the system locale (e.g. GBK on Chinese Windows), which fails with
        // "[Errno 22] Invalid argument" when stdout is a pipe (no console).
        // PYTHONUTF8=1 is not enough for the PyInstaller-built yt-dlp.exe.
        cmd.push("--encoding".to_string());
        cmd.push("utf-8".to_string());

        // Tell yt-dlp where the bundled ffmpeg lives so `bestvideo+bestaudio`
        // and `-x` merging actually work. Without this yt-dlp cannot find the
        // bundled binary and the merge fails (small / partial files).
        let ffmpeg = process::find_ffmpeg();
        if ffmpeg.exists() {
            if let Some(dir) = ffmpeg.parent() {
                cmd.push("--ffmpeg-location".to_string());
                cmd.push(dir.to_string_lossy().to_string());
            }
        }

        if let Some(proxy_url) = ProxyConfig::to_proxy_url() {
            cmd.push("--proxy".to_string());
            cmd.push(proxy_url);
        }

        // Cookies are added by the caller (fetch_video_info / download),
        // not here — avoids duplication when both the downloader state
        // and the DownloadConfig carry cookie settings.

        cmd
    }

    async fn execute_with_cookies_retry(
        &self,
        cmd: &[String],
        timeout_secs: u64,
    ) -> Result<process::CommandResult> {
        let args: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
        let result = process::execute_with_timeout(&args, timeout_secs).await?;

        if !result.is_success() {
            let stderr = result.stderr_text();
            if crate::services::cookies::CookieManager::is_chrome_lock_error(&stderr) {
                let fallback = crate::services::cookies::BROWSER_FALLBACK_ORDER
                    .iter()
                    .find(|b| {
                        let current = self.cookies_from_browser.lock().unwrap();
                        !b.eq_ignore_ascii_case(current.as_deref().unwrap_or(""))
                    })
                    .map(|b| b.to_string());

                if let Some(fb) = fallback {
                    tracing::warn!("Chrome locked, switching to {}", fb);
                    {
                        let mut c = self.cookies_from_browser.lock().unwrap();
                        *c = Some(fb);
                    }

                    let mut new_cmd: Vec<String> = cmd.to_vec();
                    let browser = self.cookies_from_browser.lock().unwrap().clone();
                    let mut found = false;
                    let mut i = 0;
                    while i < new_cmd.len() {
                        if new_cmd[i] == "--cookies-from-browser" && i + 1 < new_cmd.len() {
                            new_cmd[i + 1] = browser.clone().unwrap_or_default();
                            found = true;
                            break;
                        }
                        i += 1;
                    }
                    drop(browser);
                    if !found {
                        let browser = self.cookies_from_browser.lock().unwrap().clone();
                        new_cmd.push("--cookies-from-browser".to_string());
                        new_cmd.push(browser.unwrap_or_default());
                    }

                    let new_args: Vec<&str> = new_cmd.iter().map(|s| s.as_str()).collect();
                    return process::execute_with_timeout(&new_args, timeout_secs)
                        .await
                        .context("retry with fallback browser failed");
                }
            }
        }

        Ok(result)
    }
}

impl Default for YtDlpDownloader {
    fn default() -> Self {
        Self::new()
    }
}
