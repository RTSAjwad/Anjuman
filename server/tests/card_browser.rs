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

async fn post(app: &common::TestApp, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
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

async fn patch_json(
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
                .method("PATCH")
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

/// `sort=position` orders cards by their stored `cards.position` (the new-card
/// queue ordinal) ascending.
#[tokio::test]
async fn sort_by_position_orders_by_stored_position() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let a = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let b = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    // Give them explicit, out-of-insertion-order positions.
    sqlx::query!("UPDATE cards SET position = 200 WHERE id = $1", a)
        .execute(&app.db)
        .await
        .expect("position a");
    sqlx::query!("UPDATE cards SET position = 100 WHERE id = $1", b)
        .execute(&app.db)
        .await
        .expect("position b");

    let (status, json) = get(&app, "/cards?sort=position&page=1&per_page=50", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let cards = json["cards"].as_array().expect("cards array");
    assert_eq!(cards.len(), 2, "body: {json}");
    // b (position 100) sorts before a (position 200).
    assert_eq!(cards[0]["card_id"], b, "body: {json}");
    assert_eq!(cards[1]["card_id"], a, "body: {json}");
}

/// `status=suspended`/`status=buried` filter cards by their per-card
/// suspended/buried state (OR'd across the CSV list).
#[tokio::test]
async fn status_filter_narrows_to_suspended_and_buried() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let suspended = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let buried = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let normal = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    sqlx::query!(
        "UPDATE student_card_states SET suspended = TRUE WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        suspended
    )
    .execute(&app.db)
    .await
    .expect("suspend");
    sqlx::query!(
        "UPDATE student_card_states SET buried_at = NOW(), bury_reason = 'user_card' WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        buried
    )
    .execute(&app.db)
    .await
    .expect("bury");

    // suspended only
    let (status, json) = get(&app, "/cards?status=suspended&sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let ids: Vec<i64> = json["cards"].as_array().unwrap().iter().map(|c| c["card_id"].as_i64().unwrap()).collect();
    assert_eq!(ids, vec![suspended], "body: {json}");

    // buried only
    let (status, json) = get(&app, "/cards?status=buried&sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let ids: Vec<i64> = json["cards"].as_array().unwrap().iter().map(|c| c["card_id"].as_i64().unwrap()).collect();
    assert_eq!(ids, vec![buried], "body: {json}");

    // OR'd
    let (status, json) = get(&app, "/cards?status=suspended,buried&sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["total"], 2, "body: {json}");

    // normal card is excluded by any status filter
    let _ = normal;
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

/// `tag=…` filters cards by their note's tags (cards whose note has any of the
/// listed tags).
#[tokio::test]
async fn tag_filter_narrows_cards_by_note_tag() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let biology_card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let physics_card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    // Tag the biology card's note with "biology".
    let bio_note: i64 = sqlx::query_scalar!("SELECT note_id FROM cards WHERE id = $1", biology_card)
        .fetch_one(&app.db)
        .await
        .expect("bio note");
    let tag_id: i64 = sqlx::query_scalar!("INSERT INTO tags (name) VALUES ('biology') RETURNING id")
        .fetch_one(&app.db)
        .await
        .expect("tag");
    sqlx::query!("INSERT INTO note_tags (note_id, tag_id) VALUES ($1, $2)", bio_note, tag_id)
        .execute(&app.db)
        .await
        .expect("note tag");

    let (status, json) = get(&app, "/cards?tag=biology&sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let ids: Vec<i64> = json["cards"].as_array().unwrap().iter().map(|c| c["card_id"].as_i64().unwrap()).collect();
    assert_eq!(ids, vec![biology_card], "body: {json}");

    // The physics card is excluded.
    assert_eq!(json["total"], 1, "body: {json}");
    let _ = physics_card;
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

/// `GET /cards` carries the true note-level aggregate suspend/bury flags: `true`
/// only when *every* card of the note is suspended/buried; a single unsuspended
/// sibling keeps it `false` even though that card's own `suspended` is `true`.
#[tokio::test]
async fn cards_return_note_suspended_and_buried_flags() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    // Create a second card under the same note, so the note has two cards. The
    // `idx_cards_note_template` unique index means a note can have only one card
    // per template, so give the note type a second template first.
    let note_id: i64 = sqlx::query_scalar!("SELECT note_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("note id");
    let deck_id: i64 = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("deck id");
    let note_type_id: i64 = sqlx::query_scalar!("SELECT note_type_id FROM notes WHERE id = $1", note_id)
        .fetch_one(&app.db)
        .await
        .expect("note type id");
    let template2 = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 1, 'Card 2', '{{Back}}', '{{Front}}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert second template")
    .id;
    let card2 = sqlx::query!(
        "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3) RETURNING id",
        note_id,
        deck_id,
        template2
    )
    .fetch_one(&app.db)
    .await
    .expect("insert second card")
    .id;
    sqlx::query!(
        "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses, step_index) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0, 0)",
        teacher_id,
        card2
    )
    .execute(&app.db)
    .await
    .expect("second state");

    // Neither suspended by default.
    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let cards = json["cards"].as_array().expect("cards array");
    assert_eq!(cards.len(), 2, "body: {json}");
    for c in cards {
        assert_eq!(c["note_suspended"], false, "body: {json}");
        assert_eq!(c["note_buried"], false, "body: {json}");
    }

    // Suspend ONLY one card (a per-card op): the note is NOT fully suspended.
    sqlx::query!(
        "UPDATE student_card_states SET suspended = TRUE WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card
    )
    .execute(&app.db)
    .await
    .expect("suspend one card");

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let cards = json["cards"].as_array().expect("cards array");
    for c in cards {
        // The note is partially suspended → note_suspended stays false.
        assert_eq!(c["note_suspended"], false, "body: {json}");
        // But the suspended card's own `suspended` is true.
        if c["card_id"] == card {
            assert_eq!(c["suspended"], true, "body: {json}");
        } else {
            assert_eq!(c["suspended"], false, "body: {json}");
        }
    }

    // Suspend the second card too → now the whole note is suspended.
    sqlx::query!(
        "UPDATE student_card_states SET suspended = TRUE WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card2
    )
    .execute(&app.db)
    .await
    .expect("suspend second card");

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    for c in json["cards"].as_array().expect("cards array") {
        assert_eq!(c["note_suspended"], true, "body: {json}");
        assert_eq!(c["note_buried"], false, "body: {json}");
    }
}

/// `GET /cards` carries the note's `card_count` (number of generated cards for
/// the note), unchanged across the note's cards. A one-template note reports 1;
/// adding a second template+card reports 2.
#[tokio::test]
async fn cards_return_note_card_count() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    // One template → one card → card_count 1.
    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["cards"][0]["card_count"], 1, "body: {json}");

    // Add a second template + card under the same note → card_count 2.
    let note_id: i64 = sqlx::query_scalar!("SELECT note_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("note id");
    let deck_id: i64 = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("deck id");
    let note_type_id: i64 =
        sqlx::query_scalar!("SELECT note_type_id FROM notes WHERE id = $1", note_id)
            .fetch_one(&app.db)
            .await
            .expect("note type id");
    let template2 = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 1, 'Card 2', '{{Back}}', '{{Front}}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("second template")
    .id;
    sqlx::query!(
        "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3)",
        note_id,
        deck_id,
        template2
    )
    .execute(&app.db)
    .await
    .expect("second card");

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    for c in json["cards"].as_array().expect("cards array") {
        assert_eq!(c["card_count"], 2, "body: {json}");
    }
}

/// `GET /cards` exposes the *stored* `cards.position` for new cards (not a
/// page-local counter), and `null` for scheduled (non-new) cards — matching
/// Anki's Due column (US-4.12a).
#[tokio::test]
async fn cards_return_stored_position_for_new_cards() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // A new card and a review card.
    let new_card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let review_card = seed_studiable_card(&app, teacher_id, "review", 0, 1, 0).await;

    // Read the stored positions from the DB (the authoritative values).
    let stored_new: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", new_card)
        .fetch_one(&app.db)
        .await
        .expect("new position");

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let cards = json["cards"].as_array().expect("cards array");
    assert_eq!(cards.len(), 2, "body: {json}");

    for c in cards {
        if c["card_id"] == new_card {
            // New card → the stored position is surfaced.
            assert_eq!(c["position"], stored_new, "body: {json}");
        } else if c["card_id"] == review_card {
            // Scheduled card → no position (Anki shows an interval instead).
            assert_eq!(c["position"], serde_json::Value::Null, "body: {json}");
        }
    }
}

/// `PATCH /cards/{id}/position` repositions a new card (writes `cards.position`
/// and it survives a re-fetch), and rejects non-new cards (US-4.12b Reposition).
#[tokio::test]
async fn reposition_new_card_sets_position_and_rejects_scheduled() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    let new_card = seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let review_card = seed_studiable_card(&app, teacher_id, "review", 0, 1, 0).await;

    // Reposition the new card to position 999.
    let (status, json) = patch_json(
        &app,
        &format!("/cards/{new_card}/position"),
        &token,
        serde_json::json!({ "position": 999 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["card_id"], new_card, "body: {json}");
    assert_eq!(json["position"], 999, "body: {json}");

    // The stored position survives a re-fetch.
    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    for c in json["cards"].as_array().expect("cards array") {
        if c["card_id"] == new_card {
            assert_eq!(c["position"], 999, "body: {json}");
        }
    }

    // A review card cannot be repositioned.
    let (status, json) = patch_json(
        &app,
        &format!("/cards/{review_card}/position"),
        &token,
        serde_json::json!({ "position": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {json}");
}

/// `GET /cards` carries each card's FSRS-predicted intervals for the Grade
/// dialog (a rating→seconds map, mirroring the study screen). A review card
/// returns a non-empty map keyed by "1".."4".
#[tokio::test]
async fn cards_return_predicted_interval_for_grade_dialog() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _card = seed_studiable_card(&app, teacher_id, "review", 0, 1, 0).await;

    let (status, json) = get(&app, "/cards?sort=created_at", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    let predicted = &json["cards"][0]["predicted_interval"];
    assert!(predicted.is_object(), "predicted_interval should be a map, body: {json}");
    let obj = predicted.as_object().expect("map");
    assert!(!obj.is_empty(), "predicted_interval should be non-empty, body: {json}");
    assert!(obj.contains_key("1"), "again rating present, body: {json}");
    assert!(obj.contains_key("4"), "easy rating present, body: {json}");
}

/// `POST /cards/{id}/reset` resets a card's scheduling back to "new" in place:
/// flag is preserved, `suspended`/bury/leech are cleared, and review history is
/// kept (matching Anki's Reset, as documented in US-4.12b).
#[tokio::test]
async fn reset_card_returns_to_new_preserving_flag_and_history() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // A review card with non-trivial scheduling + a flag + a review-history row.
    let card = seed_studiable_card(&app, teacher_id, "review", 1, 5, 2).await;
    sqlx::query!(
        "UPDATE student_card_states SET flag = 3, suspended = TRUE, leech_at = NOW(), buried_at = NOW(), bury_reason = 'user_card', due_at = NOW(), stability = 3.0, difficulty = 7.0, step_index = 2 WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card
    )
    .execute(&app.db)
    .await
    .expect("seed scheduling + flag");
    sqlx::query!(
        "INSERT INTO reviews (student_id, card_id, rating, reviewed_at, state_before) VALUES ($1, $2, 4, NOW(), 'review')",
        teacher_id,
        card
    )
    .execute(&app.db)
    .await
    .expect("review row");

    let (status, json) = post(&app, &format!("/cards/{card}/reset"), &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert_eq!(json["state"], "new", "body: {json}");
    assert_eq!(json["suspended"], false, "body: {json}");
    assert_eq!(json["buried_at"], serde_json::Value::Null, "body: {json}");
    assert!(json["due_at"].is_null(), "due_at should be null, body: {json}");

    // The row is reset in place: flag preserved, scheduling zeroed.
    let flag: i64 = sqlx::query_scalar!(
        "SELECT flag FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card
    )
    .fetch_one(&app.db)
    .await
    .expect("flag");
    assert_eq!(flag, 3, "flag must be preserved by reset");

    let state: String = sqlx::query_scalar!(
        "SELECT state::text FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card
    )
    .fetch_one(&app.db)
    .await
    .expect("state")
    .unwrap_or_default();
    assert_eq!(state, "new", "state must be reset to new");

    let (suspended, leech, buried, due): (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT suspended, leech_at IS NOT NULL, buried_at IS NOT NULL, due_at IS NOT NULL FROM student_card_states WHERE student_id = $1 AND card_id = $2",
    )
    .bind(teacher_id)
    .bind(card)
    .fetch_one(&app.db)
    .await
    .expect("reset columns");
    assert!(!suspended, "reset must unsuspend");
    assert!(!leech, "reset must clear the leech marker");
    assert!(!buried, "reset must unbury");
    assert!(!due, "reset must clear due_at");

    // Review history is kept (Anki Reset; only 'Remove card history' deletes it).
    let reviews_after: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM reviews WHERE student_id = $1 AND card_id = $2",
        teacher_id,
        card
    )
    .fetch_one(&app.db)
    .await
    .expect("review count")
    .unwrap_or(0);
    assert_eq!(reviews_after, 1, "review history must be preserved");
}

/// After `reset`, the card's deck shows a `new_count` >= 1 in `GET /decks/counts`
/// (the reset card returns to the new queue). Guards the count-sync behaviour the
/// client depends on after browser reset.
#[tokio::test]
async fn reset_card_surfaces_in_deck_new_counts() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    let card = seed_studiable_card(&app, teacher_id, "review", 0, 1, 0).await;
    let deck_id: i64 = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", card)
        .fetch_one(&app.db)
        .await
        .expect("deck id");

    // Reset the review card → new queue.
    let (status, _) = post(&app, &format!("/cards/{card}/reset"), &token).await;
    assert_eq!(status, StatusCode::OK);

    // The deck's new count must now be at least 1.
    let (status, json) = get(&app, "/decks/counts", &token).await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    let deck = json["decks"]
        .as_array()
        .expect("decks array")
        .iter()
        .find(|d| d["deck_id"] == deck_id)
        .expect("missing deck in counts, body");
    assert!(
        deck["new_count"].as_i64().unwrap_or(0) >= 1,
        "reset card should count as new, body: {json}"
    );
}
