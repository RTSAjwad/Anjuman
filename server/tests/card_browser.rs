//! Tests for the card browser (`GET /cards`) — US-4.12 server side.
//!
//! The server endpoint existed but had no direct test coverage; this pins the
//! empty-filter and query-parameter paths, including the `card_state` enum →
//! text decode that formerly 500'd for the client's default (empty) filter.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{UserRole, create_user, seed_studiable_card};

async fn get(app: &common::TestApp, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// The empty-filter query (what the client sends by default) returns 200 with a
/// page, and decodes the `card_state` enum without erroring.
#[tokio::test]
async fn empty_filter_returns_page() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    let (status, json) = get(&app, "/cards?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["page"], 1);
    assert_eq!(json["total"], 1, "body: {json}");
    // The card's state is "new" (enum decoded to text).
    assert_eq!(json["cards"][0]["state"], "new", "body: {json}");
}

/// A deck filter narrows to cards in that deck.
#[tokio::test]
async fn deck_filter_narrows_results() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let card = seed_studiable_card(&app, teacher_id, "review", 0, 1, 0).await;

    // Find the card's deck id.
    let deck_id: i64 = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("deck id");

    let (status, json) = get(
        &app,
        &format!("/cards?deck_id={deck_id}&sort=created_at"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["total"], 1, "body: {json}");
    assert_eq!(json["cards"][0]["card_id"], card, "body: {json}");
}
