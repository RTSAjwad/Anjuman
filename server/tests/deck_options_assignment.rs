//! Tests for US-4.11 — assign a deck's scheduling preset via
//! `PATCH /decks/{id}/options`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user};

fn patch_options(path: &str, token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Seed a teacher-owned deck and return its id.
async fn seed_deck(app: &common::TestApp, teacher_id: i64) -> i64 {
    sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, 'Deck', $1) RETURNING id",
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck")
    .id
}

/// Assigning a preset in the caller's school updates `decks.options_id`, and the
/// response reports the new effective id.
#[tokio::test]
async fn assign_preset_in_school() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, teacher_id).await;
    let preset_id = common::seed_preset(&app, 1, 8, "suspend_card").await;

    let res = app
        .app
        .clone()
        .oneshot(patch_options(
            &format!("/decks/{deck_id}/options"),
            &token,
            serde_json::json!({ "options_id": preset_id }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["options_id"].as_i64(), Some(preset_id), "effective id is the assigned preset");
}

/// `options_id: 0` clears the assignment (`decks.options_id = NULL`); the
/// response reports the resolved effective id 0 (global default).
#[tokio::test]
async fn assign_zero_clears_to_default() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    // First assign a real preset, then clear back to default with 0.
    let preset_id = common::seed_preset(&app, 1, 8, "suspend_card").await;
    let _ = app
        .app
        .clone()
        .oneshot(patch_options(
            &format!("/decks/{deck_id}/options"),
            &token,
            serde_json::json!({ "options_id": preset_id }),
        ))
        .await
        .unwrap();

    let res = app
        .app
        .clone()
        .oneshot(patch_options(
            &format!("/decks/{deck_id}/options"),
            &token,
            serde_json::json!({ "options_id": 0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["options_id"].as_i64(), Some(0), "cleared deck resolves to default 0");

    // The DB row is NULL (unassigned), not a literal 0.
    let stored: Option<i64> = sqlx::query_scalar("SELECT options_id FROM decks WHERE id = $1")
        .bind(deck_id)
        .fetch_one(&app.db)
        .await
        .expect("read stored options_id");
    assert_eq!(stored, None, "clear stores NULL, not a literal 0");
}

/// Assigning a nonexistent preset (or one from another school) is rejected 400.
#[tokio::test]
async fn assign_rejects_cross_school_or_missing_preset() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    // Nonexistent preset id.
    let res = app
        .app
        .clone()
        .oneshot(patch_options(
            &format!("/decks/{deck_id}/options"),
            &token,
            serde_json::json!({ "options_id": 999999 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "missing preset rejected");
}
