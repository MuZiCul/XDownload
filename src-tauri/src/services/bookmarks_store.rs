//! Bookmark catalogue storage — persists every bookmark seen during a sync
//! (video and non-video alike) into the SQLite `bookmarks` table so the user
//! can browse the synced bookmarks offline and see each one's download state.
//!
//! Unlike the download history (where `video_id` is the natural primary key),
//! the bookmark catalogue is an *accumulating* list: each synced bookmark is
//! stored once per `video_id` (UNIQUE) with its own auto-increment `id`,
//! preserving insertion order across syncs.
//!
//! IMPORTANT — locking: `DB_LOCK` is a plain (non-reentrant) `Mutex`. Every
//! public entry point that needs the lock builds the `video_id → file_path`
//! map from `DownloadHistory::list()` *before* acquiring `DB_LOCK`; never call
//! `DownloadHistory::is_downloaded`/`get`/`list` while holding `DB_LOCK`, or
//! the same thread deadlocks waiting on itself.

use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::collections::HashMap;

/// One bookmark catalogue row, as returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct BookmarkRow {
    pub id: i64,
    pub video_id: String,
    pub url: String,
    pub handle: String,
    pub author_name: String,
    pub title: String,
    pub has_video: bool,
    pub downloaded: bool,
    pub added_at: i64,
}

/// Map one `bookmarks` row back into a [`BookmarkRow`].
fn row_from_sql(row: &Row<'_>) -> rusqlite::Result<BookmarkRow> {
    Ok(BookmarkRow {
        id: row.get("id")?,
        video_id: row.get("video_id")?,
        url: row.get("url")?,
        handle: row.get("handle")?,
        author_name: row.get("author_name")?,
        title: row.get("title")?,
        has_video: row.get::<_, i64>("has_video")? != 0,
        downloaded: row.get::<_, i64>("downloaded")? != 0,
        added_at: row.get("added_at")?,
    })
}

const SELECT_COLUMNS: &str = "id, video_id, url, handle, author_name, title, \
     has_video, downloaded, added_at";

/// Snapshot the download history as a `video_id → file_path` map. Must be
/// called *without* holding `DB_LOCK` (`DownloadHistory::list` takes the lock
/// itself); the returned snapshot is then used inside the locked section.
fn build_downloaded_map() -> HashMap<String, Option<String>> {
    crate::services::download_history::DownloadHistory::list()
        .into_iter()
        .map(|r| (r.video_id, r.file_path))
        .collect()
}

/// Whether a video is downloaded *and* its file still exists on disk, per the
/// snapshot built by [`build_downloaded_map`].
fn is_downloaded_in(map: &HashMap<String, Option<String>>, id: &str) -> bool {
    map.get(id)
        .and_then(|p| p.as_ref())
        .map(|p| std::path::Path::new(p).exists())
        .unwrap_or(false)
}

/// Insert or update one bookmark (keyed by `video_id`). `added_at` is only set
/// on insert — existing rows keep their first-seen time. `downloaded` is
/// supplied by the caller (computed lock-free) so this stays a pure SQL op.
fn upsert_in(
    conn: &Connection,
    b: &crate::services::bookmarks::BookmarkItem,
    now_ts: i64,
    downloaded: bool,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO bookmarks
            (video_id, url, handle, author_name, title, has_video, downloaded, added_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(video_id) DO UPDATE SET
            url=excluded.url, handle=excluded.handle, author_name=excluded.author_name,
            title=excluded.title, has_video=excluded.has_video,
            downloaded=excluded.downloaded",
        params![
            b.tweet_id,
            b.url,
            b.handle,
            b.author_name,
            b.text,
            b.has_video as i64,
            downloaded as i64,
            now_ts,
        ],
    )?;
    Ok(())
}

/// Batch-insert/refresh the bookmarks fetched in one sync (every bookmark,
/// including text/image-only ones — the `has_video` flag distinguishes them).
pub fn upsert_all(items: &[crate::services::bookmarks::BookmarkItem]) {
    // Build the download-state snapshot *before* taking the lock: computing it
    // needs `DownloadHistory::list()`, which acquires `DB_LOCK` itself, and
    // doing so while holding the lock would deadlock (non-reentrant Mutex).
    let file_map = build_downloaded_map();
    let Ok(_guard) = crate::services::db::DB_LOCK.lock() else {
        return;
    };
    let Ok(conn) = crate::services::db::open() else {
        return;
    };
    let now_ts = chrono::Utc::now().timestamp();
    for b in items {
        let downloaded = is_downloaded_in(&file_map, &b.tweet_id);
        let _ = upsert_in(&conn, b, now_ts, downloaded);
    }
}

/// List the whole bookmark catalogue, newest-first. The stored `downloaded`
/// flag is re-verified live against the download history (file may have been
/// deleted since the last sync) and written back so it stays accurate.
pub fn list() -> Vec<BookmarkRow> {
    let file_map = build_downloaded_map();
    let Ok(_guard) = crate::services::db::DB_LOCK.lock() else {
        return Vec::new();
    };
    let Ok(conn) = crate::services::db::open() else {
        return Vec::new();
    };
    list_in(&conn, &file_map).unwrap_or_default()
}

fn list_in(
    conn: &Connection,
    file_map: &HashMap<String, Option<String>>,
) -> rusqlite::Result<Vec<BookmarkRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM bookmarks ORDER BY added_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], row_from_sql)?;
    let mut out: Vec<BookmarkRow> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    // Re-verify download state live and write the flag back.
    for r in out.iter_mut() {
        let downloaded = is_downloaded_in(file_map, &r.video_id);
        if r.downloaded != downloaded {
            r.downloaded = downloaded;
            let _ = conn.execute(
                "UPDATE bookmarks SET downloaded = ?1 WHERE video_id = ?2",
                params![downloaded as i64, r.video_id],
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::bookmarks::BookmarkItem;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::services::db::create_schema(&conn).unwrap();
        conn
    }

    fn item(video_id: &str, has_video: bool) -> BookmarkItem {
        BookmarkItem {
            tweet_id: video_id.into(),
            handle: "h".into(),
            url: format!("https://x.com/h/status/{video_id}"),
            text: format!("title {video_id}"),
            author_name: "A".into(),
            has_video,
        }
    }

    #[test]
    fn upsert_inserts_and_updates_by_video_id() {
        let conn = mem_conn();
        upsert_in(&conn, &item("1", true), 100, false).unwrap();
        upsert_in(&conn, &item("2", false), 200, false).unwrap();

        let rows = list_in(&conn, &HashMap::new()).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.video_id == "1" && r.has_video));
        assert!(rows.iter().any(|r| r.video_id == "2" && !r.has_video));

        // Same video_id again: update, not duplicate; added_at unchanged.
        upsert_in(&conn, &item("1", false), 300, true).unwrap();
        let rows = list_in(&conn, &HashMap::new()).unwrap();
        assert_eq!(rows.len(), 2);
        let r1 = rows.iter().find(|r| r.video_id == "1").unwrap();
        assert!(!r1.has_video);
        assert_eq!(r1.added_at, 100); // first-seen preserved
    }

    #[test]
    fn upsert_preserves_insert_order_but_lists_newest_first() {
        let conn = mem_conn();
        upsert_in(&conn, &item("a", true), 10, false).unwrap();
        upsert_in(&conn, &item("b", true), 20, false).unwrap();
        upsert_in(&conn, &item("c", true), 30, false).unwrap();
        let ids: Vec<String> = list_in(&conn, &HashMap::new())
            .unwrap()
            .into_iter()
            .map(|r| r.video_id)
            .collect();
        assert_eq!(ids, vec!["c", "b", "a"]);
    }

    #[test]
    fn is_downloaded_in_checks_file_existence() {
        let mut map = HashMap::new();
        map.insert("hit".to_string(), None); // record exists, no file path
        map.insert("gone".to_string(), Some("Z:/definitely/not/exists.mp4".to_string()));
        assert!(!is_downloaded_in(&map, "hit"));
        assert!(!is_downloaded_in(&map, "gone"));
        assert!(!is_downloaded_in(&map, "missing"));
    }
}
