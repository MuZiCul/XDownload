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

    /// Download a video as a queue task. Returns every successfully moved
    /// file path (from `--print-to-file after_move:filepath`) — a multi-media
    /// tweet yields several files; an empty vec if the process failed without
    /// producing stderr output.
    ///
    /// Files are staged in `download_cache/{task_id}/` — every download
    /// instance gets its own directory keyed by the queue task id, so
    /// concurrent / paused / restarted tasks NEVER share cache files. This is
    /// deliberate: downloads always restart from scratch (resume disabled),
    /// and deleting an old task's cache can never touch a new download.
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

        // 缓存目录 = `{output_dir}/.xdl_cache/{task_id}`（与最终输出同盘）。
        // 下载完成后 rename 到 output_dir 是原子同盘移动（毫秒级），彻底消除
        // 跨盘 copy 的慢速/卡死/退出中断问题；下载中退出残留只在隐藏的
        // .xdl_cache 目录，启动时统一清理，不污染下载根目录。
        let cache_dir = Self::task_cache_dir(&config.output_dir, task_id);
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
        // 每次下载都从零开始：先清空自身缓存目录（防御性——task_id 目录首次
        // 存在时通常为空），pause→resume / 重试 / 重启恢复的任务一律重新下载
        // （断点续传禁用）。目录按 task_id 命名，与其他任务完全隔离。
        let _ = std::fs::remove_dir_all(cache_dir);
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
        // 禁用断点续传：即使缓存目录有残留 .part，yt-dlp 也从头下载。
        cmd.push("--no-continue".to_string());
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
            // Failure or cancellation — the staged `.part` is intentionally
            // left in the cache dir; the next attempt (retry / resume /
            // restart-restore) wipes it at the top of `run_download`, so every
            // download always starts from scratch. Partial files never leak
            // into the user-visible folder (they stay in the cache); the
            // periodic startup cleanup removes stale dirs.
            if cancel.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("用户主动取消"));
            }
            let stderr = result.stderr_text();
            if !stderr.is_empty() {
                anyhow::bail!("下载失败: {}", stderr);
            }
            return Ok(Vec::new());
        }

        // 合并/后期处理已完成，进入文件移动阶段：emit 一次进度事件清空 stage。
        // yt-dlp 合并结束后不再输出任何进度行，stage 不会自清——否则 UI 会
        // 一直显示"音视频合并"，而实际已进入跨盘移动大文件的阶段。
        (progress_cb.lock().unwrap())(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: String::new(),
            eta: String::new(),
            percent: "100%".to_string(),
            status: "moving".to_string(),
            stage: String::new(),
        });

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
        // .part are discarded with the cache wipe afterwards. 缓存与输出同盘，
        // rename 为原子移动（毫秒级），不再有跨盘 copy 慢速/中断问题。
        let output_dir_abs = Self::resolve_output_dir(&config.output_dir);
        std::fs::create_dir_all(&output_dir_abs).ok();
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
        let out = Self::resolve_output_dir(output_dir);
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

    /// 解析 output_dir 为绝对路径：相对路径基于应用根目录解析，而不是进程
    /// cwd（协议拉起应用时 cwd 可能为 system32，按 cwd 拼会把文件下载到
    /// 错误位置）。历史记录始终保存绝对路径（供 opener scope 校验）。
    fn resolve_output_dir(output_dir: &str) -> std::path::PathBuf {
        let out = std::path::Path::new(output_dir);
        if out.is_absolute() {
            out.to_path_buf()
        } else {
            crate::utils::app_home::AppHome::root().join(out)
        }
    }

    /// 任务缓存目录：`{output_dir}/.xdl_cache/{task_id}`。与最终输出同盘，
    /// 完成后 rename 原子移动；下载中残留只留在隐藏缓存目录，启动统一清理。
    fn task_cache_dir(output_dir: &str, task_id: &str) -> std::path::PathBuf {
        Self::resolve_output_dir(output_dir)
            .join(".xdl_cache")
            .join(task_id)
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
    /// fragments, info files). Called when a task is deleted / paused so its
    /// leftovers don't accumulate. The directory is keyed by `task_id`, which
    /// no other task ever reuses — a failed delete (still locked by a dying
    /// process) can never corrupt a running download; the leftover dir is
    /// simply swept by the next startup wipe.
    pub fn cleanup_task_cache(&self, config: &DownloadConfig, task_id: &str) -> bool {
        let dir = Self::task_cache_dir(&config.output_dir, task_id);
        if dir.exists() {
            tracing::info!("cleaning cache for deleted task: {}", dir.display());
            let _ = std::fs::remove_dir_all(&dir);
            true
        } else {
            false
        }
    }

    /// 清空下载缓存目录（启动时调用一次）。
    ///
    /// 缓存目录为 `{output_dir}/.xdl_cache`（下载目录内隐藏目录，与最终输出
    /// 同盘）。断点续传已禁用，残留只有异常退出/被强杀留下的 `.part` 等，
    /// 无保留价值；启动清理发生在任何任务开始之前，直接全清。同时顺手清理
    /// 旧版本遗留的应用目录缓存 `download_cache`（一次性，防历史残留堆积）。
    pub fn cleanup_download_cache() {
        let output_dir = crate::services::config::ConfigManager::load_download_dir()
            .unwrap_or_else(|| "downloads".to_string());
        Self::wipe_cache(&Self::resolve_output_dir(&output_dir).join(".xdl_cache"));
        // 旧版本缓存目录（应用根目录 download_cache）一次性清理。
        Self::wipe_cache(&crate::utils::app_home::AppHome::download_cache_dir());
    }

    /// 删除缓存目录并重建（目录不存在时跳过）。供启动清理与测试复用。
    fn wipe_cache(cache_dir: &std::path::Path) {
        if !cache_dir.exists() {
            return;
        }
        match std::fs::remove_dir_all(cache_dir) {
            Ok(()) => {
                tracing::info!("cleaned download cache: {}", cache_dir.display());
                let _ = std::fs::create_dir_all(cache_dir);
            }
            Err(e) => {
                tracing::warn!("failed to clean download cache {}: {e}", cache_dir.display());
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
    fn test_cache_cleanup_wipes_everything() {
        use std::time::SystemTime;
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

        // 缓存子目录（含嵌套残留文件）+ 散落文件 + 元数据文件，全部应被清掉。
        let task_dir = dir.join("aaaa0001");
        std::fs::create_dir_all(&task_dir).unwrap();
        std::fs::write(task_dir.join("video.part"), "partial").unwrap();
        std::fs::write(dir.join("stray.txt"), "x").unwrap();
        std::fs::write(dir.join("info.json"), "{}").unwrap();

        // 清空后目录被重建且为空（子目录 / 散落文件 / 元数据都不剩）。
        YtDlpDownloader::wipe_cache(&dir);
        assert!(dir.exists());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

        // 目录不存在时是安全的 no-op。
        let missing = dir.join("missing");
        YtDlpDownloader::wipe_cache(&missing);
        assert!(!missing.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
