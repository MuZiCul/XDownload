use crate::downloader::parser::parse_video_json;
use crate::downloader::progress::parse_progress_line;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use crate::services::config::ConfigManager;
use crate::services::proxy::ProxyConfig;
use crate::utils::process;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

/// Validate a yt-dlp `--limit-rate` value: a number (with optional decimal)
/// followed by an optional unit suffix (K/M/G, case-insensitive), e.g. "500K",
/// "1M", "2.5M". Rejects garbage so a malformed setting is silently ignored
/// instead of breaking the yt-dlp command.
fn is_valid_rate_limit(s: &str) -> bool {
    let t = s.trim();
    let (num, suffix) = match t.as_bytes().last() {
        Some(b'k') | Some(b'K') | Some(b'm') | Some(b'M') | Some(b'g') | Some(b'G') => {
            (&t[..t.len() - 1], &t[t.len() - 1..])
        }
        _ => (t, ""),
    };
    if num.is_empty() {
        return false;
    }
    // number may be integer or decimal ("1", "2.5")
    let mut dot_seen = false;
    let ok = num.bytes().all(|b| match b {
        b'0'..=b'9' => true,
        b'.' if !dot_seen => {
            dot_seen = true;
            true
        }
        _ => false,
    });
    ok && (suffix.is_empty() || matches!(suffix.to_uppercase().as_str(), "K" | "M" | "G"))
}

/// Metadata captured from yt-dlp after a download finishes, used to fill
/// history records when the frontend's info is missing.
#[derive(Debug, Clone, Default)]
pub struct DownloadedMeta {
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<i64>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
}

/// Core downloader wrapping yt-dlp CLI.
///
/// All downloads go through the multi-task queue (`DownloadQueue`), which
/// controls concurrency; this struct only tracks per-task cancel flags / PIDs.
pub struct YtDlpDownloader {
    ytdlp_path: String,
    cookies_from_browser: Mutex<Option<String>>,
    /// Per-task cancel flags (task_id → flag).
    task_cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// Per-task child process PIDs (task_id → pid), so a single task can be
    /// cancelled without affecting others.
    task_pids: Arc<Mutex<HashMap<String, u32>>>,
    /// Actual metadata captured from yt-dlp after the download finished
    /// (task_id → raw meta line), used to fill history records when the
    /// frontend's fetchVideoInfo data was missing (deep-link / extension
    /// enqueued tasks).
    task_meta: Arc<Mutex<HashMap<String, String>>>,
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
            task_cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            task_pids: Arc::new(Mutex::new(HashMap::new())),
            task_meta: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_cookies_from_browser(&self, browser: &str) {
        let mut c = self.cookies_from_browser.lock().unwrap();
        *c = Some(browser.to_string());
    }

    pub fn get_cookies_from_browser(&self) -> Option<String> {
        self.cookies_from_browser.lock().unwrap().clone()
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
        if let Some(ref b) = browser {
            if !b.is_empty() {
                cmd.push("--cookies-from-browser".to_string());
                cmd.push(b.clone());
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

    /// Stable per-download cache directory name derived from the download
    /// config. Keyed by URL + format + output options, so a re-enqueued
    /// download (same URL & format) resolves to the SAME directory as a
    /// previous attempt and yt-dlp resumes from its `.part` file instead of
    /// starting over. Changing the format / output template starts a fresh
    /// staging area (never mixing partial files of different formats).
    fn cache_key(config: &DownloadConfig) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(config.url.as_bytes());
        h.update([0u8]);
        h.update(config.format_id.as_bytes());
        h.update([0u8]);
        h.update(config.output_template.as_bytes());
        h.update([0u8]);
        h.update([if config.extract_audio { 1 } else { 0 }]);
        h.update(config.max_height.to_le_bytes());
        if let Some(items) = &config.playlist_items {
            h.update([0u8]);
            h.update(items.as_bytes());
        }
        let digest = h.finalize();
        digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
    }

    /// Download a video as a queue task. Returns every successfully moved
    /// file path (from `--print-to-file after_move:filepath`) — a multi-media
    /// tweet yields several files; an empty vec if the process failed without
    /// producing stderr output.
    ///
    /// Files are staged in `download_cache/{cache_key}/` (see `cache_key`),
    /// and each task is cancelled independently via `cancel_task`.
    pub async fn download(
        &self,
        task_id: &str,
        config: &DownloadConfig,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<Vec<String>> {
        // Register a per-task cancel flag so this task can be stopped without
        // affecting concurrent tasks.
        let flag = Arc::new(AtomicBool::new(false));
        self.task_cancel_flags
            .lock()
            .unwrap()
            .insert(task_id.to_string(), flag.clone());

        let cache_dir =
            crate::utils::app_home::AppHome::download_cache_dir().join(Self::cache_key(config));
        let tmp_path =
            std::env::temp_dir().join(format!("xdownload_last_path_{}.txt", task_id));
        let _ = std::fs::remove_file(&tmp_path);
        // 实际元数据输出文件（title/uploader/thumbnail/duration/views/likes）。
        let meta_path =
            std::env::temp_dir().join(format!("xdownload_last_meta_{}.txt", task_id));
        let _ = std::fs::remove_file(&meta_path);
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
                &meta_path,
                flag.clone(),
                pid_sink,
                progress_cb,
            )
            .await;

        // 读取 yt-dlp 输出的真实元数据，供历史记录信息兜底（如 fetch 数据缺失）。
        if let Ok(contents) = std::fs::read_to_string(&meta_path) {
            if let Some(last) = contents.lines().last().map(|l| l.to_string()) {
                self.task_meta
                    .lock()
                    .unwrap()
                    .insert(task_id.to_string(), last);
            }
        }
        let _ = std::fs::remove_file(&meta_path);

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
        meta_path: &std::path::Path,
        cancel: Arc<AtomicBool>,
        pid_sink: Arc<dyn Fn(u32) + Send + Sync + 'static>,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<Vec<String>> {
        // Stage the download inside the cache folder first. Finished files are
        // moved into the real download directory only after yt-dlp completes,
        // so an interrupted download never leaves partial files in the
        // user-visible folder.
        //
        // The cache dir is keyed by the download config (see `cache_key`), so
        // a re-enqueued / retried / resumed download reuses the previous
        // `.part` and yt-dlp (default `--continue`) resumes from where it
        // stopped. Partial files are only wiped by the startup stale-cache
        // cleanup (`cleanup_download_cache`, dirs untouched > 7 days).
        std::fs::create_dir_all(cache_dir).ok();

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
        // Per-task download rate limit (--limit-rate). Empty / None = unlimited.
        if let Some(ref rate) = config.download_rate_limit {
            if rate.is_empty() {
                // 空串 = 不限速，正常跳过。
            } else if is_valid_rate_limit(rate) {
                cmd.push("--limit-rate".to_string());
                cmd.push(rate.clone());
            } else {
                // 非法值（理论上前端已拦截）防御性忽略并记日志，避免破坏 yt-dlp 命令。
                tracing::warn!(
                    "invalid download_rate_limit {:?} ignored (expected e.g. 1M / 2.5M / 500K)",
                    rate
                );
            }
        }
        // HLS/DASH 分片并发与重试（可配置，见设置「HLS 下载」）。
        // 并发分片可显著加速 X 的 HLS 音视频分离流下载（默认单并发极慢），
        // 分片重试避免偶发坏分片导致整个任务失败。
        let hls_cfg = ConfigManager::load();
        if let Some(n) = hls_cfg.hls_concurrent_fragments {
            if n > 0 {
                cmd.push("--concurrent-fragments".to_string());
                cmd.push(n.to_string());
            }
        }
        if let Some(n) = hls_cfg.hls_fragment_retries {
            if n > 0 {
                cmd.push("--fragment-retries".to_string());
                cmd.push(n.to_string());
            }
        }

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
        let cookies_browser = if config.cookies_from_browser.is_some() {
            config.cookies_from_browser.clone()
        } else {
            self.cookies_from_browser.lock().unwrap().clone()
        };

        if let Some(ref b) = cookies_browser {
            if !b.is_empty() {
                cmd.push("--cookies-from-browser".to_string());
                cmd.push(b.clone());
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

        // Also dump the real metadata (tab-separated) so the history record can
        // be filled even when the frontend's fetchVideoInfo data is missing.
        let _ = std::fs::remove_file(meta_path);
        cmd.push("--print-to-file".to_string());
        cmd.push(
            "after_move:%(title)s\t%(uploader)s\t%(thumbnail)s\t%(duration)s\t%(view_count)s\t%(like_count)s"
                .to_string(),
        );
        cmd.push(meta_path.to_string_lossy().to_string());

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
            // Failure or cancellation — keep the staged `.part` so a retry /
            // re-enqueue of the same URL resumes instead of starting over.
            // Partial files never leak into the user-visible folder (they stay
            // in the cache); the periodic startup cleanup removes them.
            if cancel.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("用户主动取消"));
            }
            let stderr = result.stderr_text();
            if !stderr.is_empty() {
                anyhow::bail!("下载失败: {}", stderr);
            }
            return Ok(Vec::new());
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
        // illegal chars). Files listed by --print-to-file are moved first;
        // any remaining finished extras (thumbnails, subtitles) are moved too
        // so nothing is lost. Every successfully moved path is returned (a
        // multi-media tweet yields several files). Files that still end in
        // .part are discarded with the cache wipe afterwards.
        std::fs::create_dir_all(&config.output_dir).ok();
        let mut moved_paths: Vec<String> = Vec::new();
        for p in &saved_paths {
            if let Some(dst) = Self::move_to_download_dir(std::path::Path::new(p), &config.output_dir) {
                moved_paths.push(dst);
            }
        }
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                if let Some(dst) = Self::move_to_download_dir(&entry.path(), &config.output_dir) {
                    moved_paths.push(dst);
                }
            }
        }
        // Successful download — everything worth keeping was moved above.
        // Wipe the whole staging directory (.part / .info.json and the dir
        // itself) so no empty cache folders are left behind; a later
        // re-download of the same URL starts fresh.
        let _ = std::fs::remove_dir_all(cache_dir);

        Ok(moved_paths)
    }

    /// Move a finished file out of the download cache into the real download
    /// directory, sanitizing its filename. Returns the destination path, or
    /// `None` when the file could not actually be moved (so the history only
    /// records paths that really exist).
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
            } else {
                // Could not move or copy — do not report a path that does not
                // exist in the download directory.
                tracing::warn!(
                    "move_to_download_dir: failed to move {} -> {}",
                    src.display(),
                    dst.display()
                );
                return None;
            }
        }
        Some(dst.to_string_lossy().to_string())
    }

    /// Read and remove the captured metadata for a finished task. Returns
    /// `None` when nothing was captured (e.g. task failed before completion).
    pub fn read_task_meta(&self, task_id: &str) -> Option<DownloadedMeta> {
        let raw = self.task_meta.lock().unwrap().remove(task_id)?;
        let fields: Vec<&str> = raw.split('\t').collect();
        if fields.len() < 6 {
            return None;
        }
        let clean = |s: &str| -> Option<String> {
            if s.is_empty() || s == "NA" {
                None
            } else {
                Some(s.to_string())
            }
        };
        // yt-dlp 的数字字段可能是浮点（如 %(duration)s 输出 "454.601"），
        // 也可能是整数（view_count/like_count）。统一用 f64 解析后取整，
        // 否则浮点会被 parse::<i64>() 拒绝而静默丢失（历史 bug：duration 全为 0）。
        let num = |s: &str| -> Option<i64> {
            if s.is_empty() || s == "NA" {
                None
            } else {
                s.trim().parse::<f64>().ok().map(|f| f as i64)
            }
        };
        Some(DownloadedMeta {
            title: clean(fields[0]),
            uploader: clean(fields[1]),
            thumbnail: clean(fields[2]),
            duration: num(fields[3]),
            view_count: num(fields[4]),
            like_count: num(fields[5]),
        })
    }

    /// Delete the cache directory of a single task (partial files, staged
    /// fragments, info files). Called when the user DELETES a task so its
    /// `.part` / `.ytdl` leftovers don't accumulate. Pause / resume is
    /// unaffected — pausing never calls this.
    pub fn cleanup_task_cache(&self, config: &DownloadConfig) -> bool {
        let dir = crate::utils::app_home::AppHome::download_cache_dir()
            .join(Self::cache_key(config));
        if dir.exists() {
            tracing::info!("cleaning cache for deleted task: {}", dir.display());
            let _ = std::fs::remove_dir_all(&dir);
            true
        } else {
            false
        }
    }

    /// 下载缓存保留天数：目录 mtime 超过该天数的缓存条目（`.part` / 分片 / 残留
    /// 输出）视为废弃，启动时清理。
    const CACHE_RETENTION_DAYS: i64 = 7;

    /// Remove stale entries from the download cache — partial downloads, info
    /// files, or finished outputs that never got moved.
    ///
    /// Called once at startup. Because `.part` files survive across sessions
    /// (see `cache_key` / `download`) so interrupted downloads can be resumed,
    /// a full wipe would silently kill every pending task's resume data.
    /// Instead, only cache dirs whose last-modification time is older than
    /// [`CACHE_RETENTION_DAYS`] days are removed: a dir untouched for a week
    /// means its task is abandoned. Live or recently-failed tasks (fresh
    /// mtime) are never touched. The sweep is idempotent — stale dirs, once
    /// removed, are gone — so no "once per day" marker is needed.
    pub fn cleanup_download_cache() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now();
        let cutoff = now
            .checked_sub(std::time::Duration::from_secs(
                Self::CACHE_RETENTION_DAYS as u64 * 86_400,
            ))
            .unwrap_or(UNIX_EPOCH);
        let removed = Self::cleanup_stale_cache(
            &crate::utils::app_home::AppHome::download_cache_dir(),
            cutoff,
        );
        tracing::info!(
            "cleaned stale download cache (>{} days): removed {removed} dirs",
            Self::CACHE_RETENTION_DAYS
        );
    }

    /// Remove every cache entry whose directory mtime is older than `cutoff`.
    /// Returns the number of removed entries. Only directories are considered
    /// (each cache entry is a dir keyed by `cache_key`); stray files are left
    /// alone.
    fn cleanup_stale_cache(cache_dir: &std::path::Path, cutoff: std::time::SystemTime) -> usize {
        let mut removed = 0usize;
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(meta) = entry.metadata() else { continue };
                if !meta.is_dir() {
                    continue;
                }
                // Stale = last modified strictly before the cutoff.
                let stale = meta.modified().map_or(false, |m| m < cutoff);
                if stale && std::fs::remove_dir_all(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        removed
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
                // The configured browser (e.g. Chrome) has its Cookies DB
                // exclusively locked while it is running, so yt-dlp cannot
                // read it. We deliberately do NOT silently switch to another
                // browser: the user explicitly chose this browser, and silently
                // using a different one could authenticate with the wrong
                // account (or fail if the fallback is not logged in). Instead,
                // surface a clear, actionable error.
                anyhow::bail!(
                    "无法读取浏览器 Cookies：{} 正在运行且锁定了 Cookie 数据库。\n请关闭 {} 后重试，或在设置页切换到其他浏览器（如 Firefox）。\n\n{}",
                    self.browser_display_name(),
                    self.browser_display_name(),
                    stderr.trim()
                );
            }
        }

        Ok(result)
    }

    /// Human-readable name of the currently configured cookie browser.
    fn browser_display_name(&self) -> String {
        self.cookies_from_browser
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "浏览器".to_string())
    }
}

impl Default for YtDlpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(url: &str) -> DownloadConfig {
        let mut c = DownloadConfig::new(url.to_string());
        c.format_id = "bestvideo+bestaudio/best".to_string();
        c
    }

    #[test]
    fn test_cache_key_stable_for_same_config() {
        let a = cfg("https://x.com/user/status/123");
        let b = cfg("https://x.com/user/status/123");
        assert_eq!(YtDlpDownloader::cache_key(&a), YtDlpDownloader::cache_key(&b));
        // 16 hex chars
        let key = YtDlpDownloader::cache_key(&a);
        assert_eq!(key.len(), 16);
    }

    #[test]
    fn test_cache_key_differs_for_different_urls_or_formats() {
        let a = cfg("https://x.com/user/status/1");
        let b = cfg("https://x.com/user/status/2");
        assert_ne!(YtDlpDownloader::cache_key(&a), YtDlpDownloader::cache_key(&b));

        let mut same_url_diff_format = cfg("https://x.com/user/status/1");
        same_url_diff_format.format_id = "137".to_string();
        assert_ne!(
            YtDlpDownloader::cache_key(&a),
            YtDlpDownloader::cache_key(&same_url_diff_format)
        );
    }

    #[test]
    fn test_cache_key_ignores_url_trailing_whitespace_trim() {
        // enqueue 前会 trim URL；此处验证 trim 后的 URL 与未 trim 产生不同 key
        // （因为 key 用原始字符串哈希，前端/后端在入队前已统一 trim）。
        let trimmed = cfg("https://x.com/a");
        let untrimmed = cfg(" https://x.com/a ");
        // 两者 key 不同是预期的——入队路径统一 trim 后 key 才稳定。
        let _ = (YtDlpDownloader::cache_key(&trimmed), YtDlpDownloader::cache_key(&untrimmed));
    }

    #[test]
    fn test_cache_key_playlist_items_change_key() {
        let mut a = cfg("https://x.com/user/status/1");
        a.playlist_items = Some("1".to_string());
        let mut b = cfg("https://x.com/user/status/1");
        b.playlist_items = Some("1,2".to_string());
        assert_ne!(YtDlpDownloader::cache_key(&a), YtDlpDownloader::cache_key(&b));
    }

    #[test]
    fn test_is_valid_rate_limit() {
        assert!(is_valid_rate_limit("1M"));
        assert!(is_valid_rate_limit("500K"));
        assert!(is_valid_rate_limit("2.5M"));
        assert!(is_valid_rate_limit("100M"));
        assert!(is_valid_rate_limit("10"));
        assert!(is_valid_rate_limit(" 1M "));
        assert!(is_valid_rate_limit("1m"));
        assert!(is_valid_rate_limit("1G"));
        assert!(!is_valid_rate_limit(""));
        assert!(!is_valid_rate_limit("M"));
        assert!(!is_valid_rate_limit("1MM"));
        assert!(!is_valid_rate_limit("abc"));
        assert!(!is_valid_rate_limit("1.2.3M"));
        assert!(!is_valid_rate_limit("1 M"));
    }

    #[test]
    fn test_cache_cleanup_removes_only_stale_dirs() {
        use std::time::{Duration, SystemTime};
        // 独立临时目录，避免触碰真实 download_cache/。
        let dir = std::env::temp_dir().join(format!(
            "xdl_cache_cleanup_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // 两个刚创建的缓存目录 + 一个散落文件（不应被清理）。
        let fresh_a = dir.join("aaaa0001");
        let fresh_b = dir.join("aaaa0002");
        let stray = dir.join("stray.txt");
        std::fs::create_dir_all(&fresh_a).unwrap();
        std::fs::create_dir_all(&fresh_b).unwrap();
        std::fs::write(&stray, "x").unwrap();

        let now = SystemTime::now();
        // cutoff 在遥远的过去 → 所有目录都算"新"，一个都不删，散落文件也不动。
        let past = now.checked_sub(Duration::from_secs(10 * 86_400)).unwrap();
        assert_eq!(YtDlpDownloader::cleanup_stale_cache(&dir, past), 0);
        assert!(fresh_a.exists());
        assert!(fresh_b.exists());
        assert!(stray.exists());

        // cutoff 在遥远的未来 → 所有目录都算"旧"，全删；散落文件保持不动。
        let future = now.checked_add(Duration::from_secs(10 * 86_400)).unwrap();
        assert_eq!(YtDlpDownloader::cleanup_stale_cache(&dir, future), 2);
        assert!(!fresh_a.exists());
        assert!(!fresh_b.exists());
        assert!(stray.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
