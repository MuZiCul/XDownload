use serde::{Deserialize, Serialize};

/// Download progress parsed from yt-dlp --progress-template output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: i64,
    pub total_bytes: i64,
    pub speed: String,
    pub eta: String,
    pub percent: String,
    pub status: String,
}

impl DownloadProgress {
    pub fn percent_value(&self) -> f64 {
        if self.total_bytes > 0 {
            self.downloaded_bytes as f64 * 100.0 / self.total_bytes as f64
        } else {
            0.0
        }
    }
}
