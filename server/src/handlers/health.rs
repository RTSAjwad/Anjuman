// Health check handler.
//
// A simple liveness endpoint. Load balancers and monitoring tools
// hit this to verify the server is running.

use axum::Json;

use anjuman_contracts::health::Health;

/// `GET /health` — Always returns `{"status": "ok"}`.
///
/// This endpoint does not touch the database or any external service.
/// It's a simple liveness check, not a readiness check.
pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}
