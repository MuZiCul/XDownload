//! Bookmarks GraphQL queryId persistence (the `config` table in `data.db`).
//!
//! The browser extension captures the (rotating) queryId from live x.com
//! traffic and pushes it to the app via the `xdownload://setqueryid` deep
//! link. The app stores it here so bookmarks sync uses the fresh id instead
//! of the baked-in constant `DEFAULT_BOOKMARKS_QUERY_ID`.

use crate::services::db;
use anyhow::{Context, Result};
use rusqlite::params;

/// `config` table key holding the bookmarks queryId.
pub const KEY_BOOKMARKS_QUERY_ID: &str = "bookmarks_query_id";

/// Validate a captured queryId shape. X ids look like base64url tokens of
/// 8–60 chars (`[A-Za-z0-9_-]`); anything else is rejected so an arbitrary
/// deep link cannot poison the stored value.
pub fn is_valid_query_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Load the saved queryId, if any and valid.
pub fn load() -> Option<String> {
    let _guard = db::DB_LOCK.lock().unwrap();
    let conn = db::open().ok()?;
    conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        params![KEY_BOOKMARKS_QUERY_ID],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|v| is_valid_query_id(v))
}

/// Persist a captured queryId (overwrites any previous value).
pub fn save(value: &str) -> Result<()> {
    if !is_valid_query_id(value) {
        anyhow::bail!("invalid queryId: {value}");
    }
    tracing::info!("saving bookmarks queryId: {value}");
    let _guard = db::DB_LOCK.lock().unwrap();
    let conn = db::open()?;
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![KEY_BOOKMARKS_QUERY_ID, value],
    )
    .context("failed to save queryId")?;
    Ok(())
}
