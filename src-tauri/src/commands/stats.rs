//! Statistics commands — aggregate the download history table into a single
//! JSON blob for the statistics page (charts / hero numbers).

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use tracing::info;

/// Aggregate statistics derived from the `downloads` table.
///
/// All numbers are computed with a handful of SQL GROUP BY queries against the
/// existing history data; no schema changes and no writes happen here.
#[tauri::command]
pub fn get_download_stats() -> serde_json::Value {
    match compute_stats() {
        Ok(stats) => stats,
        Err(e) => {
            info!("stats query failed: {e:#}");
            json!({ "ok": false, "error": e.to_string() })
        }
    }
}

fn compute_stats() -> Result<serde_json::Value> {
    let conn = crate::services::db::open()?;

    // ---- Hero numbers ----
    let hero = conn
        .query_row(
            "SELECT
                COUNT(*) AS total,
                SUM(CASE WHEN status = 0 THEN 1 ELSE 0 END) AS success,
                SUM(CASE WHEN status = 1 THEN 1 ELSE 0 END) AS failed,
                COALESCE(SUM(CASE WHEN status = 0 THEN file_size ELSE 0 END), 0) AS total_size,
                MIN(downloaded_at) AS first_at,
                MAX(downloaded_at) AS last_at,
                COALESCE(AVG(CASE WHEN status = 0 THEN duration ELSE NULL END), 0) AS avg_duration
             FROM downloads",
            [],
            |row| {
                Ok(json!({
                    "total": row.get::<_, i64>("total")?,
                    "success": row.get::<_, i64>("success")?,
                    "failed": row.get::<_, i64>("failed")?,
                    "total_size": row.get::<_, i64>("total_size")?,
                    "first_at": row.get::<_, i64>("first_at")?,
                    "last_at": row.get::<_, i64>("last_at")?,
                    "avg_duration": row.get::<_, f64>("avg_duration")?,
                }))
            },
        )
        .context("hero query failed")?;

    // ---- Daily trend (all recorded days, local date buckets) ----
    // SQLite strftime with 'localtime' modifier converts a unix timestamp to
    // the machine's local date — matches the user-facing "today" feeling.
    let daily = {
        let mut stmt = conn
            .prepare(
                "SELECT date(downloaded_at, 'unixepoch', 'localtime') AS day,
                        COUNT(*) AS count
                 FROM downloads
                 WHERE status = 0
                 GROUP BY day
                 ORDER BY day ASC",
            )
            .context("daily trend query failed")?;
        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "name": row.get::<_, String>("day")?,
                "count": row.get::<_, i64>("count")?,
            }))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    // ---- Source distribution (numeric column mapped to readable strings) ----
    let sources = {
        let mut stmt = conn
            .prepare(
                "SELECT source, COUNT(*) AS count
                 FROM downloads
                 WHERE status = 0
                 GROUP BY source",
            )
            .context("source distribution query failed")?;
        let rows = stmt.query_map([], |row| {
            let code = row.get::<_, i64>("source")?;
            Ok(json!({
                "name": crate::services::download_history::source_name(code),
                "count": row.get::<_, i64>("count")?,
            }))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    // ---- Top uploaders (author display name) ----
    let uploaders = query_pairs(
        &conn,
        "SELECT uploader AS name, COUNT(*) AS count
         FROM downloads
         WHERE status = 0 AND uploader IS NOT NULL AND uploader <> ''
         GROUP BY uploader
         ORDER BY count DESC
         LIMIT 8",
    )
    .context("top uploaders query failed")?;

    // ---- Top handles (extracted @account) ----
    let handles = query_pairs(
        &conn,
        "SELECT handle AS name, COUNT(*) AS count
         FROM downloads
         WHERE status = 0 AND handle IS NOT NULL AND handle <> ''
         GROUP BY handle
         ORDER BY count DESC
         LIMIT 8",
    )
    .context("top handles query failed")?;

    info!(
        "stats: total={}, success={}, failed={}",
        hero["total"], hero["success"], hero["failed"]
    );
    Ok(json!({
        "ok": true,
        "hero": hero,
        "daily": daily,
        "sources": sources,
        "uploaders": uploaders,
        "handles": handles,
    }))
}

/// Run a `SELECT label, count … GROUP BY` query and return `[{key, count}, …]`.
/// The source table's `source` column needs its numeric → string mapping, so
/// it is handled separately in `compute_stats`.
fn query_pairs(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(json!({
            "name": row.get::<_, String>("name")?,
            "count": row.get::<_, i64>("count")?,
        }))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
}
