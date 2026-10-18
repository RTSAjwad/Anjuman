//! Tests for the paginated/filterable `GET /notes` (US-4.13 server side).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{UserRole, create_user};

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

/// Seed a note (deck + note type + template + card) owned by `owner_id`, and
/// return the note id.
async fn seed_note(app: &common::TestApp, owner_id: i64) -> i64 {
    let deck_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, $1, $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        owner_id
    )
    .fetch_one(&app.db)
    .await
    .expect("deck")
    .id;

    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names) VALUES (1, $1, '[\"Front\",\"Back\"]') RETURNING id",
        uuid::Uuid::new_v4().to_string()
    )
    .fetch_one(&app.db)
    .await
    .expect("note type")
    .id;

    let template_id = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 0, 'Card 1', '{{Front}}', '{{Back}}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("template")
    .id;

    let note_id = sqlx::query!(
        "INSERT INTO notes (note_type_id, fields_json) VALUES ($1, '{\"Front\":\"hola\",\"Back\":\"hello\"}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("note")
    .id;

    sqlx::query!(
        "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3)",
        note_id,
        deck_id,
        template_id
    )
    .execute(&app.db)
    .await
    .expect("card");

    note_id
}

#[tokio::test]
async fn list_notes_returns_paginated_page() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _note = seed_note(&app, teacher_id).await;

    let (status, json) = get(&app, "/notes?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["page"], 1);
    assert_eq!(json["total"], 1, "body: {json}");
    assert_eq!(json["notes"].as_array().map(|a| a.len()), Some(1), "body: {json}");
}

#[tokio::test]
async fn list_notes_filters_by_note_type() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _note = seed_note(&app, teacher_id).await;

    // Non-existent note type → empty page.
    let (status, json) = get(&app, "/notes?note_type_id=99999", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["total"], 0, "body: {json}");
}
