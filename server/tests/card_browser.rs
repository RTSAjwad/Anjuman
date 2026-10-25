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

/// `GET /cards` returns each card's note tags (joined from `note_tags`, ordered
/// by name) — additive field for the shared note-details panel (US-4.14).
#[tokio::test]
async fn cards_return_note_tags_ordered_by_name() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    // Attach tags (inserted out of order to prove the ORDER BY name in
    // `tags_for_note`); both belong to the card's note.
    let note_id: i64 = sqlx::query_scalar!("SELECT note_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("note id");
    for name in ["biology", "cells"] {
        let tag_id: i64 = sqlx::query_scalar!(
            "INSERT INTO tags (name) VALUES ($1) RETURNING id",
            name
        )
        .fetch_one(&app.db)
        .await
        .expect("tag id");
        sqlx::query!(
            "INSERT INTO note_tags (note_id, tag_id) VALUES ($1, $2)",
            note_id,
            tag_id
        )
        .execute(&app.db)
        .await
        .expect("note tag");
    }

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let tags = json["cards"][0]["tags"]
        .as_array()
        .expect("tags array")
        .iter()
        .map(|t| t["name"].as_str().expect("tag name").to_string())
        .collect::<Vec<String>>();
    assert_eq!(tags, vec!["biology".to_string(), "cells".to_string()], "body: {json}");
}

/// `GET /cards` resolves each card's `sort_field` (note type's sort field),
/// `deck_path` (full `::`-joined ancestor chain), and `due_at` (the scheduled due
/// timestamp) for the browser-table columns.
#[tokio::test]
async fn cards_return_sort_field_deck_path_and_due_at() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // Nested deck structure: parent -> child.
    let parent_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, 'Biology 101', $1) RETURNING id",
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("parent deck")
    .id;
    let child_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, parent_id, created_by) VALUES (1, 'Cell Biology', $1, $2) RETURNING id",
        parent_id,
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("child deck")
    .id;

    // Note type with an explicit sort field, a note, a card in the child deck,
    // and a scheduled due date.
    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names, sort_field) VALUES (1, $1, '[\"Front\",\"Back\"]', 'Front') RETURNING id",
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
    let card_id = sqlx::query!(
        "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3) RETURNING id",
        note_id,
        child_id,
        template_id
    )
    .fetch_one(&app.db)
    .await
    .expect("card")
    .id;
    sqlx::query!(
        "INSERT INTO student_card_states (student_id, card_id, state, due_at, stability, difficulty, reps, lapses, step_index) VALUES ($1, $2, 'review', $3, 0.0, 0.0, 1, 0, 0)",
        teacher_id,
        card_id,
        chrono::Utc::now()
    )
    .execute(&app.db)
    .await
    .expect("student state");

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let card = &json["cards"][0];
    assert_eq!(card["sort_field"], "hola", "body: {json}");
    assert_eq!(
        card["deck_path"],
        "Biology 101::Cell Biology",
        "body: {json}"
    );
    assert!(card["due_at"].is_string(), "due_at should be present, body: {json}");
}
