//! Example integration tests — proof that the harness works and the pattern
//! for stage 3 (preferences) and stage 4 (client contract) tests.
//!
//! These exercise the real router over a real Postgres test DB via
//! `tower::ServiceExt::oneshot`, with a minted JWT for auth.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user};

/// Build a `GET` request with the given bearer token.
fn get(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn me_returns_profile_for_authenticated_user() {
    let app = common::TestApp::new().await;
    let (user_id, token) = create_user(&app, UserRole::Teacher).await;

    let res = app.app.clone().oneshot(get("/me", &token)).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["id"], user_id);
    assert_eq!(json["role"], "teacher");
}

#[tokio::test]
async fn me_rejects_missing_token() {
    let app = common::TestApp::new().await;

    let res = app
        .app
        .clone()
        .oneshot(Request::builder().uri("/me").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn search_users_returns_matching_users() {
    let app = common::TestApp::new().await;
    // `/users/search` excludes admins, so seed a searchable non-admin.
    let (_, token) = create_user(&app, UserRole::Teacher).await;
    // The searching token can be any non-student; reuse the teacher's own.

    let res = app
        .app
        .clone()
        .oneshot(get("/users/search?q=Test", &token))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Our lone teacher matches the "Test" first/last name search.
    assert!(json.as_array().is_some_and(|arr| !arr.is_empty()));
}

// ---------------------------------------------------------------------------
// Stage 3 — preferences (placeholder)
// ---------------------------------------------------------------------------
//
// When `GET/PATCH /preferences` is added (ROADMAP task 3), replace the direct
// SQL below with HTTP assertions against those routes, e.g.:
//
//   GET /preferences  → 200, defaults `{"learn_ahead_seconds":1200,"day_start_hour":4}`
//   PATCH /preferences → 200, then GET reflects the upserted values.
//
// The harness already provides a real, migrated DB and a signed token, so those
// tests are straightforward additions.

#[tokio::test]
async fn preferences_default_to_anki_values_before_any_update() {
    let app = common::TestApp::new().await;
    let (user_id, _token) = create_user(&app, UserRole::Student).await;

    // Direct DB assertion for now — the endpoint does not exist yet.
    let row = sqlx::query!(
        "SELECT learn_ahead_seconds, day_start_hour FROM user_preferences WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&app.db)
    .await
    .unwrap();

    // No row yet: defaults are applied in application code, not persisted.
    assert!(row.is_none());
}
