use serde::{Deserialize, Serialize};

/// Download configuration (mirrors Java DownloadConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub url: String,
    /// Video identifier, filled by the frontend after parsing. Used to record
    /// the download in the history so repeat downloads can be detected.
    #[serde(default)]
    pub video_id: Option<String>,
    /// Video title, filled by the frontend after parsing. Stored in the
    /// download history so the history page can show a human-readable title.
    #[serde(default)]
    pub title: Option<String>,
    /// Video thumbnail URL, filled by the frontend after parsing. Stored in
    /// the download history so the history page can show a cover image.
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// Video metadata stored in the download history for display on the
    /// history page (author / duration / views / likes).
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub like_count: i64,
    #[serde(default = "default_format")]
    pub format_id: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_output_template")]
    pub output_template: String,
    #[serde(default)]
    pub extract_audio: bool,
    #[serde(default)]
    pub embed_subtitles: bool,
    #[serde(default)]
    pub embed_thumbnail: bool,
    #[serde(default)]
    pub write_thumbnail: bool,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: i32,
    #[serde(default)]
    pub cookies_from_browser: Option<String>,
    #[serde(default)]
    pub max_height: i32,
    #[serde(default)]
    pub download_archive: Option<String>,
    /// yt-dlp `--playlist-items` (e.g. "1", "1,2"). `None` downloads all media
    /// entries (e.g. every video/image in a multi-media tweet).
    #[serde(default)]
    pub playlist_items: Option<String>,
    /// Per-task download rate limit passed to yt-dlp `--limit-rate`
    /// (e.g. "1M", "500K"). `None` / empty = unlimited.
    #[serde(default)]
    pub download_rate_limit: Option<String>,
}

fn default_format() -> String {
    // Merge the best video-only + audio-only streams (X/Twitter videos are
    // split into separate streams), falling back to a single best file when no
    // mergeable pair exists. Plain `best` would pick a lower-quality single
    // file, silently downgrading the resolution.
    "bestvideo+bestaudio/best".to_string()
}
fn default_output_dir() -> String { "downloads".to_string() }
fn default_output_template() -> String { "%(title)s.%(ext)s".to_string() }
fn default_socket_timeout() -> i32 { 30 }

impl DownloadConfig {
    pub fn new(url: String) -> Self {
        Self {
            url,
            video_id: None,
            title: None,
            thumbnail: None,
            uploader: None,
            duration: 0,
            view_count: 0,
            like_count: 0,
            format_id: default_format(),
            output_dir: default_output_dir(),
            output_template: default_output_template(),
            extract_audio: false,
            embed_subtitles: false,
            embed_thumbnail: false,
            write_thumbnail: false,
            proxy: None,
            socket_timeout: default_socket_timeout(),
            cookies_from_browser: None,
            max_height: 0,
            download_archive: None,
            playlist_items: None,
            download_rate_limit: None,
        }
    }

}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Application settings stored in config/settings.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_port: Option<u32>,
    /// Proxy scheme, e.g. "http" / "socks5" / "https".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies_from_browser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Multi-task settings.
    #[serde(default)]
    pub concurrency: Option<u8>,
    #[serde(default)]
    pub retry_count: Option<u8>,
    #[serde(default)]
    pub queue_persist: Option<bool>,
    /// 隐私模式（标题遮挡 + 封面毛玻璃）。None 视为关闭（旧配置兼容）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_mode: Option<bool>,
    /// 下载限速（yt-dlp --limit-rate），如 "1M"、"25M"。None = 不限速。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_rate_limit: Option<String>,
    /// HLS/DASH 分片并发下载数（yt-dlp --concurrent-fragments）。
    /// None = 不传参（yt-dlp 默认 1）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hls_concurrent_fragments: Option<u8>,
    /// HLS/DASH 分片失败重试次数（yt-dlp --fragment-retries）。
    /// None = 不传参（yt-dlp 默认 10）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hls_fragment_retries: Option<u8>,
    /// 下载时防止系统休眠（Windows SetThreadExecutionState）。
    /// None = 关闭（旧配置兼容）。开启后：队列有活跃任务时阻止系统休眠，
    /// 全部任务结束/暂停后恢复；关闭时任何状态都不触发。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_awake: Option<bool>,
}
