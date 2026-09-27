// Database connection setup.
//
// This file handles creating the Postgres connection pool.

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Postgres, Transaction};

/// Create a connection pool to the Postgres database.
///
/// The database URL is read from the `DATABASE_URL` environment variable,
/// e.g. `postgres://127.0.0.1:5432/anjuman`.
///
/// The pool manages a small number of concurrent connections (Postgres can
/// multiplex readers and writers, so N is a concurrency ceiling, not a
/// serialization bottleneck as with SQLite).
pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await?;

    Ok(pool)
}

/// Begin a write transaction.
///
/// Postgres has no SQLite `BEGIN IMMEDIATE`; a plain `BEGIN` (read committed)
/// is sufficient for the multi-statement atomic writes in this app.
pub async fn begin_immediate(
    pool: &PgPool,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    pool.begin().await
}
