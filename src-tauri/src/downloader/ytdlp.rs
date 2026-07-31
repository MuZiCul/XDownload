use crate::downloader::parser::parse_video_json;
use crate::downloader::progress::parse_progress_line;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::models::video_info::VideoInfo;
use crate::services::proxy::ProxyConfig;
use crate::utils::process;
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

/// Core downloader wrapping yt-dlp CLI.
pub struct YtDlpDownloader {
    ytdlp_path: String,
    cookies_from_browser: Mutex<Option<String>>,
    cookies_file: Mutex<Option<String>>,
    cancel_flag: Arc<AtomicBool>,
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

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
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
        if json.is_empty() {
            anyhow::bail!("无法获取视频信息，请检查 URL 是否正确");
        }

        let json_line = json
            .lines()
            .filter(|l| l.trim().starts_with('{'))
            .last()
            .unwrap_or(&json);

        parse_video_json(json_line).map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn download(
        &self,
        config: &DownloadConfig,
        progress_cb: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<bool> {
        self.reset_cancel();

        let mut cmd = self.build_base_command();
        cmd.push("-f".to_string());
        cmd.push(config.format_id.clone());
        cmd.push("-o".to_string());
        cmd.push(config.output_path());
        cmd.push("--retries".to_string());
        cmd.push(config.retries.to_string());
        cmd.push("--socket-timeout".to_string());
        cmd.push(config.socket_timeout.to_string());
        cmd.push("--no-playlist".to_string());

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

        cmd.push(config.url.clone());

        let cancel = self.cancel_flag.clone();
        let args_refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();

        let result = process::execute_with_callbacks(
            &args_refs,
            None,  // stdout not needed — progress comes via stderr
            Some(Box::new(move |line: String| {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(progress) = parse_progress_line(&line) {
                    progress_cb(progress);
                } else if line.contains("ERROR") || line.contains("error") {
                    tracing::error!("{}", line);
                }
            })),
            None,
            false, // capture_stdout = false → avoids GBK pipe error on Windows
        )
        .await?;

        if !result.is_success() {
            let stderr = result.stderr_text();
            if !stderr.is_empty() {
                anyhow::bail!("下载失败: {}", stderr);
            }
        }

        Ok(result.is_success())
    }

    fn build_base_command(&self) -> Vec<String> {
        let mut cmd = Vec::new();
        cmd.push(self.ytdlp_path.clone());
        cmd.push("--no-warnings".to_string());
        cmd.push("--no-color".to_string());

        if ProxyConfig::is_enabled() {
            let proxy_arg = ProxyConfig::to_cli_args();
            if !proxy_arg.is_empty() {
                let parts: Vec<&str> = proxy_arg.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    cmd.push(parts[0].to_string());
                    cmd.push(parts[1].to_string());
                }
            }
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
