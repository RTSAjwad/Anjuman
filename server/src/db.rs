// Database connection setup.
//
// This file handles creating the SQLite connection pool.
// SQLite is used as an embedded database — no separate server process is needed.

use sqlx::{Sqlite, SqlitePool, Transaction, sqlite::SqlitePoolOptions};

/// Create a connection pool to the SQLite database.
///
/// The database URL is read from the `DATABASE_URL` environment variable.
/// For SQLite this is a file path, e.g. `sqlite:platform.db`.
///
/// The pool manages up to 5 concurrent connections. SQLite supports
/// multiple readers but only one writer at a time, so 5 is plenty
/// for most workloads.
pub async fn connect() -> Result<SqlitePool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // Set WAL mode and enable foreign keys once on the database file.
    // These pragmas persist across all future connections.
    sqlx::query("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}

/// Begin a write transaction using `BEGIN IMMEDIATE`.
///
/// `BEGIN IMMEDIATE` takes SQLite's write lock up front, so concurrent write
/// transactions serialise cleanly instead of one failing with `SQLITE_BUSY` at
/// commit time (which is the risk with the default `BEGIN DEFERRED`). Use this
/// for any action that issues multiple statements and must be atomic.
pub async fn begin_immediate(
    pool: &SqlitePool,
) -> Result<Transaction<'static, Sqlite>, sqlx::Error> {
    pool.begin_with("BEGIN IMMEDIATE").await
}
