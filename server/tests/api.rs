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
    let _guard = common::db_guard().await;
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
    let _guard = common::db_guard().await;
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
// Stage 3 — preferences
// ---------------------------------------------------------------------------
//
// The `GET/PATCH /preferences` HTTP round-trip tests now live in
// `server/tests/preferences.rs`.
