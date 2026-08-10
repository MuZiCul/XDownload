//! Download history — remembers which videos have been downloaded, where they
//! were saved, and when. Persisted to `config/downloads.json`.
//!
//! Used to:
//! - Show "已下载" status + download time on the video info card after parsing.
//! - Ask the user before re-downloading a video that already exists on disk.

use crate::utils::app_home::AppHome;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Outcome of a download record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Success,
    Failed,
}

impl Default for DownloadStatus {
    fn default() -> Self {
        Self::Success
    }
}

/// A single download history record (successful or failed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: String,
    pub title: Option<String>,
    /// Video thumbnail URL, shown as the cover on the history page.
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// Original video URL, used to re-download from the history page.
    #[serde(default)]
    pub url: Option<String>,
    /// Video metadata shown on the history page (author / duration / views / likes).
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub like_count: i64,
    /// Absolute path of the final saved file (may no longer exist).
    pub file_path: Option<String>,
    /// All absolute paths saved for this download (multi-media tweets produce
    /// several files; `file_path` is the first one for backward compatibility).
    #[serde(default)]
    pub file_paths: Vec<String>,
    /// File size in bytes (filled after a successful download).
    #[serde(default)]
    pub file_size: Option<i64>,
    /// Unix timestamp (seconds) of when the download completed.
    pub downloaded_at: i64,
    /// Success or failure (defaults to Success for legacy records).
    #[serde(default)]
    pub status: DownloadStatus,
    /// Failure reason (set when status = Failed).
    #[serde(default)]
    pub error: Option<String>,
    /// Number of download attempts (including retries).
    #[serde(default)]
    pub attempts: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DownloadHistoryData {
    #[serde(default)]
    records: HashMap<String, DownloadRecord>,
}

pub struct DownloadHistory;

impl DownloadHistory {
    fn history_file() -> PathBuf {
        AppHome::config_dir().join("downloads.json")
    }

    fn load_data() -> DownloadHistoryData {
        match std::fs::read_to_string(Self::history_file()) {
            Ok(json) => {
                // Tolerate a UTF-8 BOM (some editors / scripts write one), which
                // would otherwise make serde_json fail and silently clear history.
                let json = json.trim_start_matches('\u{FEFF}');
                serde_json::from_str(json).unwrap_or_default()
            }
            Err(_) => DownloadHistoryData::default(),
        }
    }

    fn save_data(data: &DownloadHistoryData) -> Result<()> {
        AppHome::ensure_config_dir().context("failed to create config dir")?;
        let json = serde_json::to_string_pretty(data).context("failed to serialize history")?;
        std::fs::write(Self::history_file(), json).context("failed to write history file")
    }

    /// Look up a download record by video id (does not check the file exists).
    pub fn get(id: &str) -> Option<DownloadRecord> {
        Self::load_data().records.get(id).cloned()
    }

    /// Whether the video was downloaded AND its file still exists on disk.
    ///
    /// This re-checks the filesystem every time, so if the user deleted the
    /// file after seeing the "已下载" hint, this returns `false` and the app
    /// will download again without asking.
    pub fn is_downloaded(id: &str) -> bool {
        match Self::get(id) {
            Some(rec) => rec
                .file_path
                .as_ref()
                .map(|p| PathBuf::from(p).exists())
                .unwrap_or(false),
            None => false,
        }
    }

    /// List all download records, most recent first.
    pub fn list() -> Vec<DownloadRecord> {
        let data = Self::load_data();
        let mut records: Vec<DownloadRecord> = data.records.into_values().collect();
        records.sort_by_key(|r| std::cmp::Reverse(r.downloaded_at));
        records
    }

    /// Remove a single record by video id.
    pub fn remove(id: &str) -> Result<()> {
        let mut data = Self::load_data();
        data.records.remove(id);
        Self::save_data(&data)
    }

    /// Remove all download records.
    pub fn clear() -> Result<()> {
        Self::save_data(&DownloadHistoryData::default())
    }

    /// Record a successful download. `file_path` is the primary path (first
    /// file); `file_paths` holds every saved file (multi-media tweets).
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        id: &str,
        title: Option<String>,
        thumbnail: Option<String>,
        url: Option<String>,
        uploader: Option<String>,
        duration: i64,
        view_count: i64,
        like_count: i64,
        file_path: Option<String>,
        file_paths: Vec<String>,
    ) -> Result<()> {
        let mut data = Self::load_data();
        data.records.insert(
            id.to_string(),
            DownloadRecord {
                id: id.to_string(),
                title,
                thumbnail,
                url,
                uploader,
                duration,
                view_count,
                like_count,
                file_path,
                file_paths,
                file_size: None,
                downloaded_at: chrono::Utc::now().timestamp(),
                status: DownloadStatus::Success,
                error: None,
                attempts: 1,
            },
        );
        Self::save_data(&data)
    }

    /// Update the file size (bytes) of an existing record after a successful
    /// download, so the history can display it.
    pub fn record_file_size(id: &str, size: i64) -> Result<()> {
        let mut data = Self::load_data();
        if let Some(rec) = data.records.get_mut(id) {
            rec.file_size = Some(size);
            Self::save_data(&data)?;
        }
        Ok(())
    }

    /// Record a failed download (after retries are exhausted).
    #[allow(clippy::too_many_arguments)]
    pub fn record_failed(
        id: &str,
        title: Option<String>,
        thumbnail: Option<String>,
        url: Option<String>,
        uploader: Option<String>,
        duration: i64,
        view_count: i64,
        like_count: i64,
        error: String,
        attempts: u8,
    ) -> Result<()> {
        let mut data = Self::load_data();
        data.records.insert(
            id.to_string(),
            DownloadRecord {
                id: id.to_string(),
                title,
                thumbnail,
                url,
                uploader,
                duration,
                view_count,
                like_count,
                file_path: None,
                file_paths: Vec::new(),
                file_size: None,
                downloaded_at: chrono::Utc::now().timestamp(),
                status: DownloadStatus::Failed,
                error: Some(error),
                attempts,
            },
        );
        Self::save_data(&data)
    }

    /// Make a filename valid on Windows while keeping it readable:
    /// - Strips characters that Windows forbids in filenames: `\ / : * ? " < > |`
    /// - Collapses runs of consecutive spaces into a single space
    ///
    /// Everything else (Chinese, letters, digits, punctuation, brackets, dots
    /// inside the name, emoji, …) is kept as-is.
    ///
    /// Only the **stem** of the filename (before the last `.`) is filtered —
    /// the extension is preserved. Pure path transformation, performs no file
    /// operations.
    ///
    /// Returns the original path unchanged when there is nothing to clean or
    /// when cleaning would produce an empty filename.
    pub fn sanitize_filename(path: &str) -> String {
        let p = PathBuf::from(path);
        let Some(file_name) = p.file_name() else {
            return path.to_string();
        };
        let name = file_name.to_string_lossy();

        // Split stem and extension at the last '.' (extension keeps its dot).
        let (stem, ext) = match name.rfind('.') {
            Some(idx) if idx > 0 => (name[..idx].to_string(), Some(name[idx..].to_string())),
            _ => (name.to_string(), None),
        };

        let mut cleaned = String::with_capacity(stem.len());
        let mut prev_space = false;
        for c in stem.chars() {
            if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                continue; // illegal on Windows — drop it
            }
            if c == ' ' {
                // Collapse consecutive spaces into a single one.
                if prev_space {
                    continue;
                }
                prev_space = true;
            } else {
                prev_space = false;
            }
            cleaned.push(c);
        }

        // Never produce an empty filename — keep the original in that case.
        if cleaned.is_empty() {
            return path.to_string();
        }

        let new_name = match ext {
            Some(e) => format!("{}{}", cleaned, e),
            None => cleaned,
        };

        if new_name == name {
            return path.to_string();
        }
        p.with_file_name(new_name).to_string_lossy().to_string()
    }
}
