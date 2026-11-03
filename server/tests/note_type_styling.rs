//! Tests for US-4.6 — note-type styling (the shared "Styling" CSS block).
//!
//! Covers: the styling is delivered on the study card, persisted on update, and
//! copied on clone.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{UserRole, create_user, seed_studiable_card};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// The study endpoint delivers the card's note-type `styling` on `StudyCard`.
#[tokio::test]
async fn study_card_exposes_note_type_styling() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, token) = create_user(&app, UserRole::Student).await;

    // Seed a studiable card, then attach styling to its note type.
    let card_id = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;

    let note_type_id = sqlx::query_scalar!(
        "SELECT nt.id FROM cards c JOIN notes n ON n.id = c.note_id JOIN note_types nt ON nt.id = n.note_type_id WHERE c.id = $1",
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("note type id");

    sqlx::query!(
        "UPDATE note_types SET styling = $1 WHERE id = $2",
        "body { color: red; }",
        note_type_id
    )
    .execute(&app.db)
    .await
    .expect("set styling");

    // The student owns the deck (created_by = student_id via seed_studiable_card),
    // so GET /decks/{id}/study is permitted. Find the deck id.
    let deck_id = sqlx::query_scalar!(
        "SELECT deck_id FROM cards WHERE id = $1",
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("deck id");

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/decks/{deck_id}/study"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let styling = json["next_card"]["styling"].as_str().unwrap();
    assert_eq!(styling, "body { color: red; }");
}

/// Updating a note type persists its `styling`, and it is returned on read-back.
#[tokio::test]
async fn update_note_type_persists_styling() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // Create a note type via clone of the seeded one is complex; insert directly
    // then PATCH it. The teacher must own it (school 1 + created_by).
    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names, sort_field, created_by) VALUES (1, $1, '[\"Front\"]', 'Front', $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        1
    )
    .fetch_one(&app.db)
    .await
    .expect("insert note type")
    .id;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/note-types/{note_type_id}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "styling": ".card { color: blue; }" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["styling"].as_str().unwrap(), ".card { color: blue; }");
}

/// Cloning a note type copies its `styling` to the copy.
#[tokio::test]
async fn clone_note_type_copies_styling() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names, sort_field, styling, created_by) VALUES (1, $1, '[\"Front\"]', 'Front', '.card {} ', $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        1
    )
    .fetch_one(&app.db)
    .await
    .expect("insert note type")
    .id;

    let source_styling = sqlx::query_scalar!(
        "SELECT styling FROM note_types WHERE id = $1",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("source styling");

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/note-types/{note_type_id}/clone"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["styling"].as_str().unwrap(), source_styling);
}
