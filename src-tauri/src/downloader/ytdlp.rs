use crate::downloader::parser::parse_video_json;
use crate::downloader::progress::parse_progress_line;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use crate::services::proxy::ProxyConfig;
use crate::utils::process;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

/// Core downloader wrapping yt-dlp CLI.
///
/// All downloads go through the multi-task queue (`DownloadQueue`), which
/// controls concurrency; this struct only tracks per-task cancel flags / PIDs.
pub struct YtDlpDownloader {
    ytdlp_path: String,
    cookies_from_browser: Mutex<Option<String>>,
    cookies_file: Mutex<Option<String>>,
    /// Per-task cancel flags (task_id → flag).
    task_cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// Per-task child process PIDs (task_id → pid), so a single task can be
    /// cancelled without affecting others.
    task_pids: Arc<Mutex<HashMap<String, u32>>>,
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
            task_cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            task_pids: Arc::new(Mutex::new(HashMap::new())),
        }
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

    /// Cancel a specific multi-task download (by task id). Only that task's
    /// process is terminated — other concurrent tasks keep running.
    pub fn cancel_task(&self, task_id: &str) {
        if let Some(flag) = self.task_cancel_flags.lock().unwrap().get(task_id) {
            flag.store(true, Ordering::SeqCst);
        }
        if let Some(pid) = self.task_pids.lock().unwrap().get(task_id).copied() {
            tracing::info!("cancel_task: killing process tree pid={}", pid);
            process::kill_process_tree(pid);
        }
    }

    pub async fn fetch_video_info(&self, url: &str) -> Result<VideoInfo> {
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

    /// Download a video as a queue task. Returns the final file path (from
    /// `--print-to-file after_move:filepath`) on success, `None` if the
    /// process failed without producing stderr output.
    ///
    /// Each task stages files in its own `download_cache/{task_id}/` directory
    /// and is cancelled independently via `cancel_task`.
    ///
    /// `preserve_cache` (task resume): keep the cache directory (and the
    /// `.part` file) so yt-dlp resumes from where it stopped instead of
    /// starting over.
    pub async fn download(
        &self,
        task_id: &str,
        preserve_cache: bool,
        config: &DownloadConfig,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<Option<String>> {
        // Register a per-task cancel flag so this task can be stopped without
        // affecting concurrent tasks.
        let flag = Arc::new(AtomicBool::new(false));
        self.task_cancel_flags
            .lock()
            .unwrap()
            .insert(task_id.to_string(), flag.clone());

        let cache_dir = crate::utils::app_home::AppHome::download_cache_dir().join(task_id);
        let tmp_path =
            std::env::temp_dir().join(format!("xdownload_last_path_{}.txt", task_id));
        let _ = std::fs::remove_file(&tmp_path);
        let pid_sink: Arc<dyn Fn(u32) + Send + Sync + 'static> = {
            let task_pids = self.task_pids.clone();
            let id_owned = task_id.to_string();
            Arc::new(move |pid| {
                task_pids.lock().unwrap().insert(id_owned.clone(), pid);
            })
        };

        let result = self
            .run_download(
                config,
                &cache_dir,
                &tmp_path,
                preserve_cache,
                flag.clone(),
                pid_sink,
                progress_cb,
            )
            .await;

        self.task_cancel_flags.lock().unwrap().remove(task_id);
        self.task_pids.lock().unwrap().remove(task_id);
        result
    }

    /// Shared download pipeline used by both the single-download path and the
    /// multi-task path. Builds the yt-dlp command, streams progress/errors,
    /// then moves finished files into the real download directory.
    #[allow(clippy::too_many_arguments)]
    async fn run_download(
        &self,
        config: &DownloadConfig,
        cache_dir: &std::path::Path,
        tmp_path: &std::path::Path,
        preserve_cache: bool,
        cancel: Arc<AtomicBool>,
        pid_sink: Arc<dyn Fn(u32) + Send + Sync + 'static>,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<Option<String>> {
        // Stage the download inside the (per-task) cache folder first. Finished
        // files are moved into the real download directory only after yt-dlp
        // completes, so an interrupted download never leaves partial files in
        // the user-visible folder — and the cache is wiped on failure/cancel.
        std::fs::create_dir_all(cache_dir).ok();
        // When resuming a paused task, keep the existing .part so yt-dlp can
        // continue from where it stopped.
        if !preserve_cache {
            Self::cleanup_cache_dir(cache_dir);
        }

        let mut cmd = self.build_base_command();
        cmd.push("-f".to_string());
        cmd.push(config.format_id.clone());
        cmd.push("-o".to_string());
        cmd.push(format!(
            "{}{}{}",
            cache_dir.to_string_lossy(),
            std::path::MAIN_SEPARATOR,
            config.output_template
        ));
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
        // Append the stream codecs so the parser can tell the video stage from
        // the audio stage of a bestvideo+bestaudio download (acodec/vcodec
        // columns); the merge stage comes from [Merger] lines.
        cmd.push(
            "download:%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._percent_str)s|%(progress.status)s|%(info.acodec)s|%(info.vcodec)s"
                .to_string(),
        );

        // Record the final file path (after all post-processing) to a temp
        // file so we can remember where the video was saved.
        let _ = std::fs::remove_file(tmp_path);
        cmd.push("--print-to-file".to_string());
        cmd.push("after_move:filepath".to_string());
        cmd.push(tmp_path.to_string_lossy().to_string());

        cmd.push(config.url.clone());

        // Diagnostic: log the full command so the actually-selected format can
        // be confirmed from the log file (used to debug resolution mismatches).
        tracing::info!("yt-dlp download command: {}", cmd.join(" "));

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
        let cancel_stdout = cancel.clone();
        let cancel_stderr = cancel.clone();
        let pid_sink_exec = pid_sink.clone();
        let result = process::execute_with_callbacks_pid(
            &args_refs,
            // stdout → informational lines + possible progress lines
            Some(Box::new(move |line: String| {
                if cancel_stdout.load(Ordering::SeqCst) {
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
                pid_sink_exec(pid);
            },
        )
        .await?;

        tracing::info!(
            "download progress lines parsed: {}",
            progress_count.load(Ordering::SeqCst)
        );

        if !result.is_success() {
            // Failure or cancellation — discard the staged cache so partial
            // files never leak into the download folder. (A resumed task keeps
            // its .part so it can be paused/resumed again.)
            if !preserve_cache {
                Self::cleanup_cache_dir(cache_dir);
            }
            // A user-initiated cancel should report a friendly message instead
            // of the raw stderr from the killed process (often empty/garbled).
            if cancel.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("用户主动取消"));
            }
            let stderr = result.stderr_text();
            if !stderr.is_empty() {
                anyhow::bail!("下载失败: {}", stderr);
            }
            return Ok(None);
        }

        // Read the actual saved paths written by --print-to-file. When multiple
        // media entries are downloaded (multi-media tweets) the file contains
        // one path per line, in download order.
        let saved_paths: Vec<String> = std::fs::read_to_string(tmp_path)
            .ok()
            .map(|s| {
                s.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Move every finished file out of the cache into the real download
        // directory, sanitizing each filename (collapse spaces, strip Windows
        // illegal chars). Files listed by --print-to-file are moved first and
        // the LAST one becomes the history path; any remaining finished extras
        // (thumbnails, subtitles) are moved too so nothing is lost. Files that
        // still end in .part are discarded with the cache wipe afterwards.
        std::fs::create_dir_all(&config.output_dir).ok();
        let mut saved_path: Option<String> = None;
        for p in &saved_paths {
            if let Some(dst) = Self::move_to_download_dir(std::path::Path::new(p), &config.output_dir) {
                saved_path = Some(dst);
            }
        }
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let _ = Self::move_to_download_dir(&entry.path(), &config.output_dir);
            }
        }
        // Nothing finished should remain; wipe whatever is left (.part, info).
        Self::cleanup_cache_dir(cache_dir);

        Ok(saved_path)
    }

    /// Move a finished file out of the download cache into the real download
    /// directory, sanitizing its filename. Returns the destination path.
    fn move_to_download_dir(src: &std::path::Path, output_dir: &str) -> Option<String> {
        if !src.is_file() {
            return None;
        }
        if src.extension().and_then(|e| e.to_str()) == Some("part") {
            return None;
        }
        // 相对 output_dir 转绝对路径，历史记录保存绝对路径（供 opener scope 校验）。
        // 相对路径基于应用根目录解析，而不是进程 cwd：协议拉起应用时 cwd 可能
        // 是 system32，按 cwd 拼会把文件下载到错误位置。
        let out = std::path::Path::new(output_dir);
        let out = if out.is_absolute() {
            out.to_path_buf()
        } else {
            crate::utils::app_home::AppHome::root().join(out)
        };
        let target = out.join(src.file_name().unwrap_or_default());
        let dst = std::path::PathBuf::from(
            crate::services::download_history::DownloadHistory::sanitize_filename(
                &target.to_string_lossy(),
            ),
        );
        tracing::info!("moving {} -> {}", src.display(), dst.display());
        if std::fs::rename(src, &dst).is_err() {
            // Cross-device move or locked target — fall back to copy + remove.
            if std::fs::copy(src, &dst).is_ok() {
                let _ = std::fs::remove_file(src);
            }
        }
        Some(dst.to_string_lossy().to_string())
    }

    /// Remove everything currently staged in the download cache — partial
    /// downloads, info files, or finished outputs that never got moved.
    /// Called at startup (to wipe leftovers from a previous session).
    pub fn cleanup_download_cache() {
        Self::cleanup_cache_dir(&crate::utils::app_home::AppHome::download_cache_dir());
    }

    /// Remove all contents of a cache directory (files and subdirectories),
    /// keeping the directory itself. Used per-task so concurrent downloads
    /// never touch each other's staged files.
    fn cleanup_cache_dir(dir: &std::path::Path) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
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
        // and `-x` merging actually work. Only the bundled binary is used —
        // download.rs already rejects the download when it is missing.
        let ffmpeg = process::bundled_ffmpeg_path();
        if let Some(dir) = ffmpeg.parent() {
            cmd.push("--ffmpeg-location".to_string());
            cmd.push(dir.to_string_lossy().to_string());
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
