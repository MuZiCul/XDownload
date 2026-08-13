//! Shared SQLite database access for XDownload.
//!
//! Single file `config/data.db` holding all app tables (`downloads`, `config`,
//! …). Every call opens a fresh connection under the global [`DB_LOCK`], so
//! concurrent writers (download history, config keys, …) are serialized and
//! never hit `SQLITE_BUSY`.

use crate::utils::app_home::AppHome;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// Serializes every database access (a fresh connection is opened per call).
pub static DB_LOCK: Mutex<()> = Mutex::new(());

/// SQLite database file (`config/data.db`).
pub fn db_path() -> PathBuf {
    AppHome::config_dir().join("data.db")
}

/// Open the database, creating `config/` and all tables on first use.
pub fn open() -> Result<Connection> {
    AppHome::ensure_config_dir().context("failed to create config dir")?;
    let conn = Connection::open(db_path()).context("failed to open database")?;
    create_schema(&conn).context("failed to create database schema")?;
    Ok(conn)
}

/// Create all tables and indexes (idempotent, safe to run on every open).
pub fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS downloads (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             video_id      TEXT NOT NULL UNIQUE,
             title         TEXT,
             thumbnail     TEXT,
             url           TEXT,
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
             attempts      INTEGER NOT NULL DEFAULT 1
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
    )
}
