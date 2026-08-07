use serde::{Deserialize, Serialize};

/// Video information parsed from yt-dlp --dump-json output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoInfo {
    /// Video identifier (set by parser; empty when unknown).
    #[serde(default)]
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub like_count: i64,
    #[serde(default)]
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub formats: Vec<Format>,
    /// Number of media entries in this URL (e.g. a tweet containing several
    /// videos/images). 1 for a normal single-media video.
    #[serde(default)]
    pub media_count: usize,
    // ===== Download status (filled by the command layer, not by yt-dlp) =====
    /// Whether this video has already been downloaded (record exists AND the
    /// saved file still exists on disk).
    #[serde(default)]
    pub downloaded: bool,
    /// Unix timestamp of the previous download, if any.
    #[serde(default)]
    pub downloaded_at: Option<i64>,
    /// Absolute path of the previously saved file, if any.
    #[serde(default)]
    pub download_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Format {
    pub format_id: String,
    #[serde(default)]
    pub ext: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub filesize: Option<i64>,
    #[serde(default, alias = "filesize_approx")]
    pub filesize_approx: Option<i64>,
    #[serde(default)]
    pub tbr: Option<f64>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
    #[serde(default)]
    pub format_note: Option<String>,
}

impl Format {
    pub fn has_video(&self) -> bool {
        self.vcodec.as_deref().map_or(false, |v| v != "none" && !v.is_empty())
    }

    pub fn has_audio(&self) -> bool {
        self.acodec.as_deref().map_or(false, |a| a != "none" && !a.is_empty())
    }

    pub fn file_size(&self) -> i64 {
        self.filesize.or(self.filesize_approx).unwrap_or(0)
    }
}

impl VideoInfo {
    pub fn get_best_format(&self) -> Option<&Format> {
        self.formats.iter().max_by_key(|f| {
            let mut score = 0i64;
            if f.has_video() {
                score += f.height.unwrap_or(0) as i64;
            }
            if f.has_audio() {
                score += 1000;
            }
            score
        })
    }

    pub fn get_format_by_height(&self, max_height: i32) -> Option<&Format> {
        self.formats
            .iter()
            .filter(|f| f.has_video() && f.has_audio() && f.height.unwrap_or(9999) <= max_height)
            .max_by_key(|f| f.height.unwrap_or(0))
    }
}

/// Format duration in seconds to hh:mm:ss or mm:ss string
pub fn format_duration(seconds: i64) -> String {
    if seconds <= 0 {
        return "?".to_string();
    }
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

/// Format large numbers with Chinese-style units (or just comma-separated)
pub fn format_number(n: i64) -> String {
    if n >= 100_000_000 {
        format!("{:.1}亿", n as f64 / 100_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}万", n as f64 / 10_000.0)
    } else {
        n.to_string()
    }
}

pub fn format_file_size(bytes: i64) -> String {
    if bytes <= 0 {
        return "?".to_string();
    }
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{} B", bytes)
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", b / (1024.0 * 1024.0 * 1024.0))
    }
}
