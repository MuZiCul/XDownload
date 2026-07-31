use serde::{Deserialize, Serialize};

/// Download configuration (mirrors Java DownloadConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub url: String,
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
    #[serde(default = "default_retries")]
    pub retries: i32,
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: i32,
    #[serde(default)]
    pub cookies_file: Option<String>,
    #[serde(default)]
    pub cookies_from_browser: Option<String>,
    #[serde(default)]
    pub max_height: i32,
    #[serde(default)]
    pub download_archive: Option<String>,
}

fn default_format() -> String { "best".to_string() }
fn default_output_dir() -> String { "downloads".to_string() }
fn default_output_template() -> String { "%(title)s.%(ext)s".to_string() }
fn default_retries() -> i32 { 5 }
fn default_socket_timeout() -> i32 { 30 }

impl DownloadConfig {
    pub fn new(url: String) -> Self {
        Self {
            url,
            format_id: default_format(),
            output_dir: default_output_dir(),
            output_template: default_output_template(),
            extract_audio: false,
            embed_subtitles: false,
            embed_thumbnail: false,
            write_thumbnail: false,
            proxy: None,
            retries: default_retries(),
            socket_timeout: default_socket_timeout(),
            cookies_file: None,
            cookies_from_browser: None,
            max_height: 0,
            download_archive: None,
        }
    }

    pub fn output_path(&self) -> String {
        let dir = if self.output_dir.ends_with('/') || self.output_dir.ends_with('\\') {
            self.output_dir.clone()
        } else {
            format!("{}{}", self.output_dir, std::path::MAIN_SEPARATOR)
        };
        format!("{}{}", dir, self.output_template)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies_from_browser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}
