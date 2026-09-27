// Shared application state.
//
// `AppState` is injected into every request handler by Axum.
// It holds long-lived resources that all handlers need — currently
// just the database connection pool.
//
// Because `PgPool` is `Clone` (it's backed by an `Arc` internally),
// `AppState` can also be `Clone`. Axum clones it once per request so
// handlers can access it without any locking.

use sqlx::PgPool;

/// The application state available to every request handler.
#[derive(Clone)]
pub struct AppState {
    /// The Postgres connection pool.
    ///
    /// Handlers borrow connections from this pool to run queries.
    pub db: PgPool,
}
