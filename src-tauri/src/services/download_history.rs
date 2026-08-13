//! Download history — remembers which videos have been downloaded, where they
//! were saved, and when. Persisted to the SQLite database `config/data.db`
//! (table `downloads`).
//!
//! Used to:
//! - Show "已下载" status + download time on the video info card after parsing.
//! - Ask the user before re-downloading a video that already exists on disk.
//!
//! Note: history lives only in SQLite; the legacy `config/downloads.json` is
//! no longer read or migrated (left untouched on disk as a manual backup).

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

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
///
/// `video_id` is the public unique key (the tweet status id for X videos).
/// The `downloads` table also has an internal auto-increment `id` column that
/// is *not* exposed here — it exists purely for database row management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    /// Video id (X tweet status id).
    pub video_id: String,
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

/// SQL status column: 0 = Success, 1 = Failed.
fn status_code(s: DownloadStatus) -> i64 {
    match s {
        DownloadStatus::Success => 0,
        DownloadStatus::Failed => 1,
    }
}

/// Map one `downloads` row back into a [`DownloadRecord`].
fn record_from_row(row: &Row<'_>) -> rusqlite::Result<DownloadRecord> {
    let file_paths_json: Option<String> = row.get("file_paths")?;
    let status: i64 = row.get("status")?;
    Ok(DownloadRecord {
        video_id: row.get("video_id")?,
        title: row.get("title")?,
        thumbnail: row.get("thumbnail")?,
        url: row.get("url")?,
        uploader: row.get("uploader")?,
        duration: row.get("duration")?,
        view_count: row.get("view_count")?,
        like_count: row.get("like_count")?,
        file_path: row.get("file_path")?,
        file_paths: serde_json::from_str(file_paths_json.as_deref().unwrap_or("[]"))
            .unwrap_or_default(),
        file_size: row.get("file_size")?,
        downloaded_at: row.get("downloaded_at")?,
        status: if status == 1 {
            DownloadStatus::Failed
        } else {
            DownloadStatus::Success
        },
        error: row.get("error")?,
        attempts: row.get::<_, i64>("attempts")? as u8,
    })
}

const SELECT_COLUMNS: &str = "video_id, title, thumbnail, url, uploader, duration, \
     view_count, like_count, file_path, file_paths, file_size, downloaded_at, \
     status, error, attempts";

pub struct DownloadHistory;

impl DownloadHistory {
    /// Initialize the database, creating `config/` and all tables on first
    /// use. Call once at application startup. Failures are logged but never
    /// panic — history degrades to empty.
    pub fn init() {
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        if let Err(e) = crate::services::db::open() {
            warn!("failed to initialize database: {e:#}");
        }
    }

    /// Look up a download record by video id (does not check the file exists).
    pub fn get(video_id: &str) -> Option<DownloadRecord> {
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open().ok()?;
        Self::get_in(&conn, video_id).ok().flatten()
    }

    fn get_in(conn: &Connection, video_id: &str) -> rusqlite::Result<Option<DownloadRecord>> {
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM downloads WHERE video_id = ?1"),
            params![video_id],
            record_from_row,
        )
        .optional()
    }

    /// Whether the video was downloaded AND its file still exists on disk.
    ///
    /// This re-checks the filesystem every time, so if the user deleted the
    /// file after seeing the "已下载" hint, this returns `false` and the app
    /// will download again without asking.
    pub fn is_downloaded(video_id: &str) -> bool {
        match Self::get(video_id) {
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
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let Ok(conn) = crate::services::db::open() else {
            return Vec::new();
        };
        Self::list_in(&conn).unwrap_or_default()
    }

    fn list_in(conn: &Connection) -> rusqlite::Result<Vec<DownloadRecord>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM downloads ORDER BY downloaded_at DESC"
        ))?;
        let rows = stmt.query_map([], record_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
    }

    /// Remove a single record by video id.
    pub fn remove(video_id: &str) -> Result<()> {
        info!("removing download history record: video_id={video_id}");
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open()?;
        let changed = conn.execute(
            "DELETE FROM downloads WHERE video_id = ?1",
            params![video_id],
        )?;
        if changed == 0 {
            warn!("download history record not found: video_id={video_id}");
        }
        Ok(())
    }

    /// Remove all download records.
    pub fn clear() -> Result<()> {
        info!("clearing all download history");
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open()?;
        conn.execute("DELETE FROM downloads", [])?;
        Ok(())
    }

    /// Record a successful download. `file_path` is the primary path (first
    /// file); `file_paths` holds every saved file (multi-media tweets).
    /// A record with the same `video_id` is overwritten (last download wins).
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        video_id: &str,
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
        info!("recording download success: video_id={video_id}, file_count={}", file_paths.len());
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open()?;
        record_in(
            &conn,
            video_id,
            title,
            thumbnail,
            url,
            uploader,
            duration,
            view_count,
            like_count,
            file_path,
            file_paths,
            None, // file_size — 下载成功后由 record_file_size 单独回填
            chrono::Utc::now().timestamp(),
            DownloadStatus::Success,
            None,
            1,
        )
        .map_err(Into::into)
    }

    /// Update the file size (bytes) of an existing record after a successful
    /// download, so the history can display it.
    pub fn record_file_size(video_id: &str, size: i64) -> Result<()> {
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open()?;
        conn.execute(
            "UPDATE downloads SET file_size = ?1 WHERE video_id = ?2",
            params![size, video_id],
        )?;
        Ok(())
    }

    /// Record a failed download (after retries are exhausted).
    #[allow(clippy::too_many_arguments)]
    pub fn record_failed(
        video_id: &str,
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
        info!("recording download failure: video_id={video_id}, attempts={attempts}");
        let _guard = crate::services::db::DB_LOCK.lock().unwrap();
        let conn = crate::services::db::open()?;
        record_in(
            &conn,
            video_id,
            title,
            thumbnail,
            url,
            uploader,
            duration,
            view_count,
            like_count,
            None,
            Vec::new(),
            None, // file_size
            chrono::Utc::now().timestamp(),
            DownloadStatus::Failed,
            Some(error),
            attempts,
        )
        .map_err(Into::into)
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

/// Insert or update one record (shared by `record` and `record_failed`).
/// `downloaded_at` / `status` / `attempts` are supplied by the caller.
#[allow(clippy::too_many_arguments)]
fn record_in(
    conn: &Connection,
    video_id: &str,
    title: Option<String>,
    thumbnail: Option<String>,
    url: Option<String>,
    uploader: Option<String>,
    duration: i64,
    view_count: i64,
    like_count: i64,
    file_path: Option<String>,
    file_paths: Vec<String>,
    file_size: Option<i64>,
    downloaded_at: i64,
    status: DownloadStatus,
    error: Option<String>,
    attempts: u8,
) -> rusqlite::Result<()> {
    let file_paths_json = serde_json::to_string(&file_paths).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO downloads
            (video_id, title, thumbnail, url, uploader, duration, view_count,
             like_count, file_path, file_paths, file_size, downloaded_at,
             status, error, attempts)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(video_id) DO UPDATE SET
             title=excluded.title, thumbnail=excluded.thumbnail,
             url=excluded.url, uploader=excluded.uploader,
             duration=excluded.duration, view_count=excluded.view_count,
             like_count=excluded.like_count, file_path=excluded.file_path,
             file_paths=excluded.file_paths, file_size=excluded.file_size,
             downloaded_at=excluded.downloaded_at, status=excluded.status,
             error=excluded.error, attempts=excluded.attempts",
        params![
            video_id,
            title,
            thumbnail,
            url,
            uploader,
            duration,
            view_count,
            like_count,
            file_path,
            file_paths_json,
            file_size,
            downloaded_at,
            status_code(status),
            error,
            attempts as i64,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // ---- Sanitize (unchanged legacy behavior) ----

    #[test]
    fn test_sanitize_removes_windows_illegal_chars() {
        // 注意：Windows 上 PathBuf 会把 `:` 当作盘符前缀、`\` 当作路径分隔符，
        // 因此这里用其它非法字符（* ? " < > |）验证清洗逻辑。
        assert_eq!(
            DownloadHistory::sanitize_filename(r#"a*b*c?d"e<f>g|h.mp4"#),
            "abcdefgh.mp4"
        );
    }

    #[test]
    fn test_sanitize_collapses_spaces() {
        assert_eq!(
            DownloadHistory::sanitize_filename("hello   world  .mp4"),
            "hello world .mp4"
        );
        assert_eq!(DownloadHistory::sanitize_filename("a    b.mp4"), "a b.mp4");
    }

    #[test]
    fn test_sanitize_keeps_chinese_and_extension() {
        assert_eq!(
            DownloadHistory::sanitize_filename("我的 视频 (官方).mp4"),
            "我的 视频 (官方).mp4"
        );
        // `|`（Windows 非法字符）被移除，扩展名保留。
        assert_eq!(
            DownloadHistory::sanitize_filename("视频|剪辑.重制.mp4"),
            "视频剪辑.重制.mp4"
        );
    }

    #[test]
    fn test_sanitize_keeps_valid_name_unchanged() {
        let name = "simple-name_1#+.mp4";
        assert_eq!(DownloadHistory::sanitize_filename(name), name);
    }

    #[test]
    fn test_sanitize_empty_result_returns_original() {
        assert_eq!(
            DownloadHistory::sanitize_filename(r#"::**??.mp4"#),
            r#"::**??.mp4"#
        );
    }

    #[test]
    fn test_sanitize_collapses_spaces_without_extension() {
        assert_eq!(
            DownloadHistory::sanitize_filename("file  name"),
            "file name"
        );
    }

    #[test]
    fn test_sanitize_path_preserves_directories() {
        assert_eq!(
            DownloadHistory::sanitize_filename(r#"D:\Downloads\a b:c.mp4"#),
            r#"D:\Downloads\a bc.mp4"#
        );
    }

    // ---- SQLite CRUD (in-memory, never touches the real config dir) ----

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::services::db::create_schema(&conn).unwrap();
        conn
    }

    fn sample_record(video_id: &str) -> DownloadRecord {
        DownloadRecord {
            video_id: video_id.to_string(),
            title: Some(format!("title {video_id}")),
            thumbnail: Some(format!("https://example.com/{video_id}.jpg")),
            url: Some(format!("https://x.com/u/status/{video_id}")),
            uploader: Some("author".to_string()),
            duration: 123,
            view_count: 1000,
            like_count: 42,
            file_path: Some(r#"D:\Downloads\t.mp4"#.to_string()),
            file_paths: vec![
                r#"D:\Downloads\t.mp4"#.to_string(),
                r#"D:\Downloads\t2.mp4"#.to_string(),
            ],
            file_size: Some(2048),
            downloaded_at: 1_700_000_000,
            status: DownloadStatus::Success,
            error: None,
            attempts: 1,
        }
    }

    #[test]
    fn record_and_get_roundtrip() {
        let conn = mem_conn();
        let rec = sample_record("111");
        record_in(
            &conn,
            &rec.video_id,
            rec.title.clone(),
            rec.thumbnail.clone(),
            rec.url.clone(),
            rec.uploader.clone(),
            rec.duration,
            rec.view_count,
            rec.like_count,
            rec.file_path.clone(),
            rec.file_paths.clone(),
            rec.file_size,
            rec.downloaded_at,
            rec.status,
            rec.error.clone(),
            rec.attempts,
        )
        .unwrap();

        let got = DownloadHistory::get_in(&conn, "111").unwrap().unwrap();
        assert_eq!(got.video_id, "111");
        assert_eq!(got.title, rec.title);
        assert_eq!(got.thumbnail, rec.thumbnail);
        assert_eq!(got.url, rec.url);
        assert_eq!(got.uploader, rec.uploader);
        assert_eq!(got.duration, 123);
        assert_eq!(got.view_count, 1000);
        assert_eq!(got.like_count, 42);
        assert_eq!(got.file_path, rec.file_path);
        assert_eq!(got.file_paths, rec.file_paths);
        assert_eq!(got.file_size, Some(2048));
        assert_eq!(got.downloaded_at, 1_700_000_000);
        assert_eq!(got.status, DownloadStatus::Success);
        assert_eq!(got.attempts, 1);

        // Unknown id → None.
        assert!(DownloadHistory::get_in(&conn, "nope").unwrap().is_none());
    }

    #[test]
    fn record_overwrites_same_video_id() {
        let conn = mem_conn();
        let rec = sample_record("111");
        record_in(
            &conn, "111", None, None, None, None, 0, 0, 0, None, vec![], None, 5,
            DownloadStatus::Failed, Some("boom".to_string()), 3,
        )
        .unwrap();
        // Keep the sample in between to prove the count stays at 1.
        record_in(
            &conn,
            &rec.video_id,
            rec.title.clone(),
            None,
            None,
            None,
            rec.duration,
            0,
            0,
            None,
            vec![],
            None,
            rec.downloaded_at,
            DownloadStatus::Success,
            None,
            1,
        )
        .unwrap();

        let got = DownloadHistory::get_in(&conn, "111").unwrap().unwrap();
        assert_eq!(got.title, rec.title);
        assert_eq!(got.status, DownloadStatus::Success);
        assert_eq!(DownloadHistory::list_in(&conn).unwrap().len(), 1);
    }

    #[test]
    fn list_orders_by_downloaded_at_desc() {
        let conn = mem_conn();
        record_in(&conn, "a", None, None, None, None, 0, 0, 0, None, vec![], None, 100, DownloadStatus::Success, None, 1).unwrap();
        record_in(&conn, "b", None, None, None, None, 0, 0, 0, None, vec![], None, 200, DownloadStatus::Success, None, 1).unwrap();
        record_in(&conn, "c", None, None, None, None, 0, 0, 0, None, vec![], None, 150, DownloadStatus::Success, None, 1).unwrap();

        let ids: Vec<String> = DownloadHistory::list_in(&conn)
            .unwrap()
            .into_iter()
            .map(|r| r.video_id)
            .collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn remove_and_clear_work() {
        let conn = mem_conn();
        record_in(&conn, "a", None, None, None, None, 0, 0, 0, None, vec![], None, 1, DownloadStatus::Success, None, 1).unwrap();
        record_in(&conn, "b", None, None, None, None, 0, 0, 0, None, vec![], None, 2, DownloadStatus::Success, None, 1).unwrap();

        conn.execute("DELETE FROM downloads WHERE video_id = ?1", params!["a"]).unwrap();
        assert!(DownloadHistory::get_in(&conn, "a").unwrap().is_none());
        assert!(DownloadHistory::get_in(&conn, "b").unwrap().is_some());

        conn.execute("DELETE FROM downloads", []).unwrap();
        assert!(DownloadHistory::list_in(&conn).unwrap().is_empty());
    }

    #[test]
    fn failed_record_status_roundtrip() {
        let conn = mem_conn();
        record_in(
            &conn, "f", Some("t".into()), None, None, None, 0, 0, 0, None, vec![],
            None, 42, DownloadStatus::Failed, Some("err".into()), 3,
        )
        .unwrap();
        let got = DownloadHistory::get_in(&conn, "f").unwrap().unwrap();
        assert_eq!(got.status, DownloadStatus::Failed);
        assert_eq!(got.error.as_deref(), Some("err"));
        assert_eq!(got.attempts, 3);
    }

    // ---- Legacy JSON migration ----

    #[test]
    fn schema_supports_duplicate_table_creation() {
        // create_schema 必须幂等（每次 open 都会执行）。
        let conn = mem_conn();
        crate::services::db::create_schema(&conn).unwrap();
        crate::services::db::create_schema(&conn).unwrap();
    }

    #[test]
    fn db_path_is_under_config_dir() {
        let p = crate::services::db::db_path();
        assert!(p.ends_with("data.db"));
        assert!(p.to_string_lossy().contains("config"));
        let _ = p; // 只断言路径形态，不实际创建文件。
    }
}
