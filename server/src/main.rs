// anjuman_server — main entry point
//
// The server is split into a library (`lib.rs`, testable by `server/tests/`)
// and this thin binary. The binary owns process concerns: tracing init, a
// background token-cleanup task, and binding the TCP listener.

use std::net::SocketAddr;

use anjuman_server::app::app;
use anjuman_server::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = anjuman_server::db::connect()
        .await
        .expect("database connection failed");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("migration failed");

    // -------------------------------------------------------------------
    // Background cleanup task
    // -------------------------------------------------------------------

    let cleanup_db = db.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        loop {
            let now = chrono::Utc::now();

            match sqlx::query!("DELETE FROM revoked_tokens WHERE expires_at <= $1", now)
                .execute(&cleanup_db)
                .await
            {
                Ok(result) => {
                    let deleted = result.rows_affected();
                    if deleted > 0 {
                        tracing::info!("Cleaned up {deleted} expired revoked token(s)");
                    }
                }
                Err(e) => {
                    tracing::error!("Token cleanup failed: {e}");
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });

    // -------------------------------------------------------------------
    // Start the server
    // -------------------------------------------------------------------

    let state = AppState { db };
    let app = app(state);

    // Bind to 127.0.0.1 by default for security.
    // Pass `--bind 0.0.0.0` to allow connections from other devices.
    let args: Vec<String> = std::env::args().collect();
    let bind_ip = args
        .iter()
        .position(|a| a == "--bind")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("127.0.0.1");

    let addr: SocketAddr = format!("{bind_ip}:3000")
        .parse()
        .expect("invalid bind address");
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
