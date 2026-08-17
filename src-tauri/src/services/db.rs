//! Shared SQLite database access for XDownload.
//!
//! Single file `config/data.db` holding all app tables (`downloads`, `config`,
//! `bookmarks`, …). A small r2d2 connection pool is created once (lazily) and
//! reused: every public entry point that reads or writes the database goes
//! through [`open`], which borrows a connection from the pool for the duration
//! of the call. SQLite is opened in WAL mode, so multiple readers run
//! concurrently and never block the writer; write/write contention is handled
//! by `busy_timeout` instead of a global lock.

use crate::utils::app_home::AppHome;
use anyhow::{Context, Result};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// Max connections in the pool. SQLite still serializes writers (single
/// writer + `busy_timeout`), so a modest size gives UI reads real
/// concurrency while keeping write/write collisions rare.
const POOL_MAX_SIZE: u32 = 5;

/// Lazily-initialized connection pool. The pool — and with it the schema and
/// WAL setup — is created exactly once, then every caller briefly borrows a
/// connection. There is no global mutex: readers are concurrent.
static DB_POOL: OnceLock<Result<r2d2::Pool<SqliteConnectionManager>, anyhow::Error>> = OnceLock::new();

/// SQLite database file (`config/data.db`).
pub fn db_path() -> PathBuf {
    AppHome::config_dir().join("data.db")
}

/// Borrow a database connection from the pool, creating `config/`, the schema
/// and the pool on first use. The returned handle dereferences to
/// [`rusqlite::Connection`]; drop it when the read/write section is done so
/// the connection returns to the pool.
///
/// Initialization failure is remembered permanently and reported on every
/// call (a broken pool is not retried until restart — preferable to failing
/// inside a hot loop). Never panics.
pub fn open() -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
    let pool = DB_POOL.get_or_init(|| init());
    match pool {
        Ok(p) => p.get().map_err(|e| {
            tracing::warn!("[XDownload] db: failed to borrow connection: {e}");
            anyhow::anyhow!("failed to borrow database connection: {e}")
        }),
        Err(e) => {
            tracing::warn!("[XDownload] db: pool not initialized: {e:#}");
            Err(anyhow::anyhow!("database not initialized: {e:#}"))
        }
    }
}

/// Create the pool: config dir, per-connection PRAGMAs (`with_init`, applied
/// to every new connection) and the schema. Runs exactly once on first access
/// (see [`open`]).
fn init() -> Result<r2d2::Pool<SqliteConnectionManager>> {
    AppHome::ensure_config_dir().context("failed to create config dir")?;
    let manager = SqliteConnectionManager::file(db_path()).with_init(init_pragmas);
    let pool = r2d2::Pool::builder()
        .max_size(POOL_MAX_SIZE)
        .min_idle(Some(1))
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .context("failed to build database connection pool")?;
    // Schema is created exactly once, on the first pooled connection.
    let init_conn = pool.get().context("failed to open initial database connection")?;
    create_schema(&init_conn)?;
    Ok(pool)
}

/// Per-connection PRAGMAs, run by r2d2 `with_init` for every new connection.
/// WAL keeps readers from blocking the writer; `busy_timeout` turns transient
/// lock contention into a wait instead of an immediate `SQLITE_BUSY`;
/// `synchronous = NORMAL` is the recommended WAL setting (durable enough for
/// this app, much faster than FULL).
///
/// `busy_timeout` must come *before* `journal_mode`: on first pool creation a
/// background (min_idle) connection and the init connection may set up WAL
/// concurrently, and the loser must wait (up to 5s) instead of failing with an
/// immediate `SQLITE_BUSY` before the timeout is installed.
fn init_pragmas(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA busy_timeout = 5000;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )
}

/// Create all tables and indexes (idempotent, runs once on first open).
pub fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS downloads (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             video_id      TEXT NOT NULL UNIQUE,
             title         TEXT,
             thumbnail     TEXT,
             url           TEXT,
             handle        TEXT,
             uploader      TEXT,
             duration      INTEGER NOT NULL DEFAULT 0,
             view_count    INTEGER NOT NULL DEFAULT 0,
             like_count    INTEGER NOT NULL DEFAULT 0,
             file_path     TEXT,
             file_paths    TEXT,
             file_size     INTEGER,
             downloaded_at INTEGER NOT NULL DEFAULT 0,
             status        INTEGER NOT NULL DEFAULT 0,
             error         TEXT,
             attempts      INTEGER NOT NULL DEFAULT 1,
             source        INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS idx_downloads_downloaded_at
             ON downloads(downloaded_at);
         CREATE TABLE IF NOT EXISTS config (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS bookmarks (
             id          INTEGER PRIMARY KEY AUTOINCREMENT,
             video_id    TEXT NOT NULL UNIQUE,
             url         TEXT NOT NULL,
             handle      TEXT NOT NULL DEFAULT '',
             author_name TEXT NOT NULL DEFAULT '',
             title       TEXT NOT NULL DEFAULT '',
             has_video   INTEGER NOT NULL DEFAULT 0,
             downloaded  INTEGER NOT NULL DEFAULT 0,
             added_at    INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS idx_bookmarks_added_at
             ON bookmarks(added_at DESC);",
         )?;

         // 幂等迁移：老库（2026-08-13 SQLite 迁移后、source 列加入前创建的
         // downloads 表）缺少 `source` 列，而查询语句（download_history.rs 的
         // SELECT_COLUMNS）包含 source → 查询 prepare 失败 → 历史页静默空。
         // CREATE TABLE IF NOT EXISTS 不会给已存在的表补列，必须显式 ALTER。
         let has_source = {
         let mut stmt = conn.prepare(
             "SELECT COUNT(*) FROM pragma_table_info('downloads') WHERE name = 'source'",
         )?;
         stmt.query_row([], |row| row.get::<_, i64>(0))?
         };
         if has_source == 0 {
         conn.execute_batch("ALTER TABLE downloads ADD COLUMN source INTEGER NOT NULL DEFAULT 0;")?;
         }

         // 幂等迁移：老库（handle 列加入前创建的 downloads 表）缺少 `handle` 列，
         // 同样需要显式 ALTER（CREATE TABLE IF NOT EXISTS 不会给已存在的表补列）。
         let has_handle = {
         let mut stmt = conn.prepare(
             "SELECT COUNT(*) FROM pragma_table_info('downloads') WHERE name = 'handle'",
         )?;
         stmt.query_row([], |row| row.get::<_, i64>(0))?
         };
         if has_handle == 0 {
         conn.execute_batch("ALTER TABLE downloads ADD COLUMN handle TEXT;")?;
         }

         Ok(())
         }

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: the pool applies `with_init` PRAGMAs to every connection
    /// and can hand out multiple connections concurrently (the read
    /// concurrency the second step of the DB optimization is about). Uses an
    /// in-memory manager so the real `config/data.db` is never touched.
    #[test]
    fn pool_serves_concurrent_borrows_and_applies_init() -> Result<()> {
        let manager = SqliteConnectionManager::memory().with_init(init_pragmas);
        let pool = r2d2::Pool::builder()
            .max_size(3)
            .build(manager)
            .context("failed to build pool")?;

        // First borrow: schema creation works and a write/read round-trips.
        let a = pool.get()?;
        create_schema(&a)?;
        a.execute("INSERT INTO config (key, value) VALUES ('k', 'v')", [])?;
        let v: String = a.query_row("SELECT value FROM config WHERE key = 'k'", [], |r| r.get(0))?;
        assert_eq!(v, "v");

        // Second borrow while the first is still held, and `with_init`
        // PRAGMAs must have been applied to it as well.
        let b = pool.get()?;
        let to: i64 = b.query_row("PRAGMA busy_timeout", [], |r| r.get(0))?;
        assert_eq!(to, 5000);
        Ok(())
    }
}
