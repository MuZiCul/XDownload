use crate::services::proxy::ProxyConfig;
use crate::utils::app_home::AppHome;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Global cancel flag for bootstrap downloads.
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

/// 流式下载空闲超时（秒）：连续超过该时长未收到任何数据视为卡死。
/// 只拦截"完全没数据"的僵死连接；持续有数据（哪怕极慢）不超时。
const IDLE_READ_TIMEOUT: u64 = 60;

/// Signal cancellation of the current bootstrap download.
pub fn cancel_download() {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
}

fn reset_cancel() {
    CANCEL_FLAG.store(false, Ordering::SeqCst);
}

fn is_cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::SeqCst)
}

/// Bootstrap downloads missing tool binaries (yt-dlp, ffmpeg)
/// on first run with multi-source fallback.
pub struct Bootstrap;

/// yt-dlp download sources (direct then mirror).
const YTDLP_URLS: &[&str] = &[
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
];

/// ffmpeg download sources.
/// 仅使用 GitHub（BtbN 官方认可的 Windows 构建，GitHub CDN 通常比官网快）。
/// 资产名 `ffmpeg-master-latest-win64-gpl.zip` 由自动构建 workflow 固定命名，
/// 每次构建覆盖同名资产，不随版本号变化。
const FFMPEG_URLS: &[&str] = &[
    "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip",
];

impl Bootstrap {
    /// Build a direct (no-proxy) client with a fast-failing connect timeout
    /// (8s) so a blocked local network fails quickly before falling back to
    /// the configured proxy. No overall request timeout is set — stalled
    /// reads are handled per-chunk in `download_to_file` (idle timeout), so a
    /// slow-but-progressing download is never cut off.
    fn build_direct_client() -> Result<reqwest::Client> {
        // 版本号单一数据源 = Cargo.toml（CARGO_PKG_VERSION 编译期注入）。
        let ua = format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) XDownload/{}",
            env!("CARGO_PKG_VERSION")
        );
        reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(8))
            .user_agent(ua)
            .build()
            .context("failed to build direct HTTP client")
    }

    /// Build a client routed through the configured proxy, or None when no
    /// proxy is configured.
    fn build_proxy_client() -> Result<Option<reqwest::Client>> {
        let Some(proxy) = ProxyConfig::to_reqwest_proxy() else {
            return Ok(None);
        };
        let ua = format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) XDownload/{}",
            env!("CARGO_PKG_VERSION")
        );
        reqwest::Client::builder()
            .proxy(proxy)
            .user_agent(ua)
            .build()
            .map(Some)
            .context("failed to build proxy HTTP client")
    }

    /// Download a file from `url` to `dest` with progress reporting.
    ///
    /// Strategy (when `force_mode` is `None`): try the local network **direct**
    /// first (fast-failing connect timeout), then fall back to the configured
    /// proxy when the direct attempt fails. Without a configured proxy only the
    /// direct attempt runs.
    ///
    /// `force_mode` overrides the strategy:
    /// - `Some("proxy")`: only use the configured proxy; fail fast when no
    ///   proxy is configured (UI disables the switch before this can happen).
    /// - `Some("direct")`: only use the direct connection.
    /// - `None`: automatic (direct first, proxy fallback).
    ///
    /// `mode_cb` is invoked with `"direct"` / `"proxy"` each time the active
    /// network path changes, so the UI can show the current mode in real time.
    pub async fn download_with_fallback(
        url: &str,
        dest: &Path,
        progress_cb: &impl Fn(u32),
        mode_cb: &impl Fn(&str),
        force_mode: Option<&str>,
    ) -> Result<()> {
        match force_mode {
            Some("proxy") => {
                // 强制代理：直接走代理，不再尝试直连。
                let Some(proxy_client) = Self::build_proxy_client()? else {
                    return Err(anyhow::anyhow!("未配置代理，无法使用代理下载: {}", url));
                };
                mode_cb("proxy");
                return Self::download_to_file(&proxy_client, url, dest, progress_cb)
                    .await
                    .map_err(|e| anyhow::anyhow!("代理下载失败: {}: {}", url, e));
            }
            Some("direct") => {
                let direct = Self::build_direct_client()?;
                mode_cb("direct");
                return Self::download_to_file(&direct, url, dest, progress_cb).await;
            }
            _ => {}
        }

        // 自动：直连优先，失败切代理。
        let direct = Self::build_direct_client()?;
        mode_cb("direct");
        match Self::download_to_file(&direct, url, dest, progress_cb).await {
            Ok(()) => return Ok(()),
            Err(direct_err) => {
                // 用户取消 → 直接透传，不再尝试代理（避免多余的代理请求，
                // 也保证前端能识别"下载已取消"而静默关闭而非弹下载失败）。
                if is_cancelled() {
                    return Err(direct_err);
                }
                tracing::warn!(
                    "direct download failed ({}), trying configured proxy",
                    url
                );
                if let Some(proxy_client) = Self::build_proxy_client()? {
                    mode_cb("proxy");
                    return Self::download_to_file(&proxy_client, url, dest, progress_cb)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("直连与代理下载均失败: {}: {}", url, e)
                        });
                }
                Err(direct_err)
            }
        }
    }

    // ==================== yt-dlp ====================

    /// Download yt-dlp.exe into bin/ with progress reporting.
    /// Tries each source URL in turn. Returns the path to the downloaded binary.
    ///
    /// `mode_cb` receives `"direct"` / `"proxy"` when the active network path
    /// changes (see `download_with_fallback`).
    ///
    /// `force_mode` — `Some("proxy")`/`Some("direct")` force the download path,
    /// `None` keeps the automatic direct-then-proxy fallback.
    pub async fn download_ytdlp(
        progress_cb: impl Fn(u32),
        mode_cb: &impl Fn(&str),
        force_mode: Option<&str>,
    ) -> Result<PathBuf> {
        reset_cancel();
        let bin_dir = AppHome::bin_dir();
        std::fs::create_dir_all(&bin_dir)
            .context("failed to create bin directory")?;

        let dest = if cfg!(windows) {
            bin_dir.join("yt-dlp.exe")
        } else {
            bin_dir.join("yt-dlp")
        };

        let mut last_error: Option<anyhow::Error> = None;
        for (i, url) in YTDLP_URLS.iter().enumerate() {
            if i > 0 {
                tracing::info!("switching to mirror source {} for yt-dlp", i + 1);
            }

            match Self::download_with_fallback(url, &dest, &progress_cb, mode_cb, force_mode).await {
                Ok(_) => {
                    // Set executable on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Ok(meta) = std::fs::metadata(&dest) {
                            let mut perms = meta.permissions();
                            perms.set_mode(0o755);
                            let _ = std::fs::set_permissions(&dest, perms);
                        }
                    }

                    // Validate the downloaded binary
                    if Self::validate_ytdlp(&dest).await {
                        return Ok(dest);
                    }
                    tracing::warn!("downloaded yt-dlp binary failed validation, trying next source...");
                    let _ = std::fs::remove_file(&dest);
                    last_error = Some(anyhow!("binary validation failed"));
                }
                Err(e) => {
                    tracing::warn!("yt-dlp source {} failed: {}", i + 1, e);
                    let _ = std::fs::remove_file(&dest);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("no yt-dlp download sources configured")))
    }

    /// Validate a yt-dlp binary by running --version.
    pub async fn validate_ytdlp(path: &Path) -> bool {
        tokio::process::Command::new(path)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    // ==================== ffmpeg ====================

    /// Download ffmpeg zip, extract ffmpeg.exe / ffprobe.exe / ffplay.exe into bin/.
    /// Tries each source URL in turn. Returns the path to ffmpeg.exe on success.
    ///
    /// `on_extracting` is called once before zip extraction begins (so the UI can
    /// switch from "downloading" to "extracting" display).
    ///
    /// `mode_cb` receives `"direct"` / `"proxy"` when the active network path
    /// changes (see `download_with_fallback`).
    ///
    /// `force_mode` — `Some("proxy")`/`Some("direct")` force the download path,
    /// `None` keeps the automatic direct-then-proxy fallback.
    pub async fn download_ffmpeg(
        progress_cb: impl Fn(u32),
        on_extracting: impl FnOnce(),
        mode_cb: &impl Fn(&str),
        force_mode: Option<&str>,
    ) -> Result<PathBuf> {
        reset_cancel();
        let bin_dir = AppHome::bin_dir();
        std::fs::create_dir_all(&bin_dir)
            .context("failed to create bin directory")?;

        let temp_zip = bin_dir.join("ffmpeg-temp.zip");

        let mut on_extracting = Some(on_extracting);
        let mut last_error: Option<anyhow::Error> = None;
        for (i, url) in FFMPEG_URLS.iter().enumerate() {
            if i > 0 {
                tracing::info!("switching to mirror source {} for ffmpeg", i + 1);
            }

            match Self::download_with_fallback(url, &temp_zip, &progress_cb, mode_cb, force_mode)
                .await
            {
                Ok(_) => {
                    if let Some(cb) = on_extracting.take() {
                        cb();
                    }
                    match Self::extract_ffmpeg(&temp_zip, &bin_dir) {
                        Ok(ffmpeg_path) => {
                            let _ = std::fs::remove_file(&temp_zip);
                            return Ok(ffmpeg_path);
                        }
                        Err(e) => {
                            tracing::warn!("ffmpeg extraction failed: {}", e);
                            let _ = std::fs::remove_file(&temp_zip);
                            last_error = Some(e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("ffmpeg source {} failed: {}", i + 1, e);
                    let _ = std::fs::remove_file(&temp_zip);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("no ffmpeg download sources configured")))
    }

    /// Extract ffmpeg.exe, ffprobe.exe, ffplay.exe from a zip archive into the target dir.
    // TODO: Use spawn_blocking for CPU-bound work if needed.
fn extract_ffmpeg(zip_path: &Path, dest_dir: &Path) -> Result<PathBuf> {
        let file = std::fs::File::open(zip_path)
            .context("failed to open ffmpeg zip")?;

        let mut archive = zip::ZipArchive::new(file)
            .context("failed to open zip archive")?;

        let mut ffmpeg_path = None;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .context("failed to read zip entry")?;

            let name = entry.name().to_lowercase();

            // We only extract exe files from the bin/ folder
            let is_target = (name.ends_with("ffmpeg.exe")
                || name.ends_with("ffprobe.exe")
                || name.ends_with("ffplay.exe"))
                && name.contains("bin/");

            if !is_target {
                continue;
            }

            // Get just the filename (last component)
            let file_name = Path::new(entry.name())
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| entry.name().to_string());

            let dest = dest_dir.join(&file_name);
            tracing::info!("extracting: {}", file_name);

            let mut out = std::fs::File::create(&dest)
                .with_context(|| format!("failed to create {}", dest.display()))?;

            std::io::copy(&mut entry, &mut out)
                .with_context(|| format!("failed to extract {}", file_name))?;

            // 恢复 zip 内原始修改时间作为文件 mtime：BtbN master 构建 zip 内
            // 文件时间戳 ≈ 构建打包时刻（≈ GitHub 的 published_at），这样更新
            // 检测的"本地构建时间"基准语义正确，不会因"下载时刻"与"构建时刻"
            // 不同而持续误报更新。
            // zip::DateTime → SystemTime。zip 2.x 推荐 `TryFrom<zip::DateTime>
            // for time::OffsetDateTime`，但 time crate 是 tauri 的间接依赖、
            // 无法直接命名目标类型，故用弃用的 `to_time()`（功能相同）。
            #[allow(deprecated)]
            if let Some(odt) = entry.last_modified().and_then(|dt| dt.to_time().ok()) {
                use std::fs::FileTimes;
                let ts: std::time::SystemTime = odt.into();
                let times = FileTimes::new().set_modified(ts);
                let _ = out.set_times(times);
            }

            // Track ffmpeg.exe path for the return value
            if file_name.eq_ignore_ascii_case("ffmpeg.exe") {
                ffmpeg_path = Some(dest);
            }

            // Set executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&dest) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&dest, perms);
                }
            }
        }

        ffmpeg_path.ok_or_else(|| anyhow!("ffmpeg.exe not found in downloaded zip archive"))
    }

    // ==================== Core Download Logic ====================

    /// Download a file from a URL to a destination path, reporting progress.
    async fn download_to_file(
        client: &reqwest::Client,
        url: &str,
        dest: &Path,
        progress_cb: &impl Fn(u32),
    ) -> Result<()> {
        let response = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {} failed", url))?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 302 && status.as_u16() != 301 {
            return Err(anyhow!("HTTP {} for {}", status.as_u16(), url));
        }

        let total = response.content_length().unwrap_or(0);

        // Stream the response chunk by chunk, reporting progress
        let tmp = dest.with_extension("tmp");
        // 清理上次可能残留的 tmp（幂等）。
        let _ = std::fs::remove_file(&tmp);

        // 内部下载块：任何失败路径（超时 / chunk 读取 / 写入 / 取消 / flush）
        // 都会返回 Err；外层统一删除 tmp，避免残留半成品文件。
        let download_result = async {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::File::create(&tmp)
                .await
                .context("failed to create output file")?;

            let mut downloaded: u64 = 0;
            let mut last_pct: u32 = 0;
            let mut stream = response.bytes_stream();

            use futures_util::StreamExt;
            loop {
                // 空闲超时：取代客户端整体 180s 总超时。持续有数据不超时，
                // 只有超过 IDLE_READ_TIMEOUT 秒未收到任何数据才判卡死。
                let next_chunk =
                    tokio::time::timeout(Duration::from_secs(IDLE_READ_TIMEOUT), stream.next())
                        .await
                        .map_err(|_| {
                            anyhow!(
                                "下载超时：{IDLE_READ_TIMEOUT} 秒未收到数据，请检查网络后重试"
                            )
                        })?;
                let Some(chunk) = next_chunk else { break };
                // Check cancel flag between chunks
                if is_cancelled() {
                    return Err(anyhow!("下载已取消"));
                }

                let chunk = chunk.context("failed to read response chunk")?;
                file.write_all(&chunk)
                    .await
                    .context("failed to write chunk to file")?;
                downloaded += chunk.len() as u64;

                if total > 0 {
                    let pct = ((downloaded as f64 / total as f64) * 100.0) as u32;
                    let pct = pct.min(99); // reserve 100 for completion
                    if pct > last_pct {
                        last_pct = pct;
                        progress_cb(pct);
                    }
                }
            }

            file.flush().await.context("failed to flush output file")?;
            drop(file);
            Ok(())
        }
        .await;

        if let Err(e) = download_result {
            // 失败统一清理 tmp（覆盖超时/读错/写错/取消/flush 全部路径）。
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }

        // Atomic rename from tmp → final destination
        std::fs::rename(&tmp, dest)
            .context("failed to rename tmp file to final destination")?;

        // Report 100% on completion
        progress_cb(100);

        Ok(())
    }
}
