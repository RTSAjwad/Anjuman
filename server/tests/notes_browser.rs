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

async fn post_json(
    app: &common::TestApp,
    path: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
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

/// A note's `sort_field` is resolved server-side from `fields[effective_sort_field]`,
/// falling back to the first field when the note type has no explicit sort field.
#[tokio::test]
async fn note_response_sort_field_resolves_to_first_field_fallback() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _note = seed_note(&app, teacher_id).await;

    let (status, json) = get(&app, "/notes?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let note = &json["notes"][0];
    // The seed note type has no sort_field, so Anki's first-field fallback applies.
    assert_eq!(
        note["sort_field"],
        "hola",
        "first-field fallback should resolve to the 'Front' value, body: {json}"
    );
}

/// `GET /notes` carries the caller's own note-level suspend state: `false` with
/// no (or partial) suspension, `true` only when *every* card is suspended.
#[tokio::test]
async fn note_response_carries_note_suspended_and_buried() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note = seed_note(&app, teacher_id).await;

    let card_id: i64 = sqlx::query_scalar!("SELECT id FROM cards WHERE note_id = $1", note)
        .fetch_one(&app.db)
        .await
        .expect("card id");

    // No scheduling state yet → note is neither suspended nor buried.
    let (status, json) = get(&app, "/notes?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["notes"][0]["note_suspended"], false, "body: {json}");
    assert_eq!(json["notes"][0]["note_buried"], false, "body: {json}");

    // Suspend the sole card for the caller → the whole note is suspended.
    sqlx::query!(
        "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses, step_index, suspended) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0, 0, TRUE) ON CONFLICT (student_id, card_id) DO UPDATE SET suspended = TRUE",
        teacher_id,
        card_id
    )
    .execute(&app.db)
    .await
    .expect("suspend");

    let (status, json) = get(&app, "/notes?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["notes"][0]["note_suspended"], true, "body: {json}");
    assert_eq!(json["notes"][0]["note_buried"], false, "body: {json}");
}

/// A note's card `front`/`back` are wrapped in `.card` at render time (US-4.6.1),
/// so the note type's styling (which targets `.card`) actually applies.
#[tokio::test]
async fn note_cards_wrap_front_and_back_in_card_wrapper() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _note = seed_note(&app, teacher_id).await;

    let (status, json) = get(&app, "/notes?sort=created_at&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let card = &json["notes"][0]["cards"][0];
    assert_eq!(
        card["front"],
        "<div class=\"card\">hola</div>",
        "front should be wrapped in a .card element, body: {json}"
    );
    assert_eq!(
        card["back"],
        "<div class=\"card\">hello</div>",
        "back should be wrapped in a .card element, body: {json}"
    );
}

/// `POST /notes` creates a note and attaches the requested tags (normalised +
/// deduped), returning them on the response; the note's deck is the first card's
/// deck (duplicate-note prefill).
#[tokio::test]
async fn create_note_with_tags_attaches_them_and_returns_deck() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    // Reuse the seeded note's note type + deck for the create request.
    let note_type_id: i64 = sqlx::query_scalar!("SELECT note_type_id FROM notes WHERE id = $1", note_id)
        .fetch_one(&app.db)
        .await
        .expect("note type id");
    let deck_id: i64 = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE note_id = $1", note_id)
        .fetch_one(&app.db)
        .await
        .expect("deck id");

    let (status, json) = post_json(
        &app,
        "/notes",
        &token,
        serde_json::json!({
            "note_type_id": note_type_id,
            "deck_id": deck_id,
            "fields": {"Front": "bonjour", "Back": "hello"},
            "tags": ["Biology", "biology", " Cells "]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {json}");

    // Tags are normalised (trim + lowercase) and deduped.
    let tags = json["tags"]
        .as_array()
        .expect("tags array")
        .iter()
        .map(|t| t["name"].as_str().expect("tag name").to_string())
        .collect::<Vec<String>>();
    assert_eq!(tags, vec!["biology".to_string(), "cells".to_string()], "body: {json}");

    // The note's deck is its (first/only) card's deck.
    let card_deck = json["cards"][0]["deck_id"].as_i64().unwrap();
    assert_eq!(card_deck, deck_id, "body: {json}");
}
