//! Tests for US-4.8 — teachers/admins can study decks, with real per-state counts.
//!
//! Verifies `list_decks` and `GET /decks/counts` return real per-state counts for
//! teachers/admins (mirroring students), and that `studyable` reflects the actual
//! grant (via the US-4.7 predicate) rather than a hardcoded `true`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user, seed_studiable_card};

fn get(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

async fn list_decks(app: &common::TestApp, token: &str) -> Vec<serde_json::Value> {
    let res = app.app.clone().oneshot(get("/decks", token)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// A teacher who owns a deck with a new card sees real per-state counts and
/// `studyable: true`.
#[tokio::test]
async fn teacher_owner_gets_real_counts_and_studyable() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // Seed one new card (with its own deck + note) owned by this teacher.
    seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    let decks = list_decks(&app, &token).await;
    assert_eq!(decks.len(), 1, "teacher sees their owned deck");

    let deck = &decks[0];
    assert_eq!(deck["studyable"], true, "owner deck is studyable");
    assert!(deck["new_count"].as_i64().unwrap() >= 1, "new card counted");
}

/// A teacher who does not own/collaborate a deck does not see it (scoped out).
#[tokio::test]
async fn teacher_sees_only_owned_or_collaborated_decks() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _otok) = create_user(&app, UserRole::Teacher).await;
    let (_other_id, other_tok) = create_user(&app, UserRole::Teacher).await;
    seed_studiable_card(&app, owner_id, "new", 0, 0, 0).await;

    let decks = list_decks(&app, &other_tok).await;
    assert!(decks.is_empty(), "non-owner/non-collab teacher sees nothing");
}

/// An admin sees the deck with `studyable` reflecting the *grant* (false when
/// they don't own/collaborate), not a hardcoded true.
#[tokio::test]
async fn admin_studyable_reflects_grant_not_blanket() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (_admin_id, atok) = create_user(&app, UserRole::Admin).await;
    seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    let decks = list_decks(&app, &atok).await;
    assert_eq!(decks.len(), 1, "admin sees all school decks");
    assert_eq!(decks[0]["studyable"], false, "admin without grant is not studyable");
}

/// `GET /decks/counts` returns real per-state counts for a teacher/admin who
/// holds the deck (not hardcoded 0).
#[tokio::test]
async fn teacher_admin_deck_counts_are_real() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;

    let res = app
        .app
        .clone()
        .oneshot(get("/decks/counts", &token))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let deck = &json["decks"][0];
    assert_eq!(deck["new_count"].as_i64().unwrap() >= 1, true, "per-state new count is real");
}

/// `DeckResponse.options_id` is the *effective* preset id (US-4.9): a deck with
/// no preset (`decks.options_id = NULL`) reports `0` (the global default), and a
/// deck assigned a preset reports that preset's id.
#[tokio::test]
async fn deck_options_id_resolves_to_effective_preset() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // A deck with no preset (seed_studiable_card inserts a deck with NULL
    // options_id) -> options_id reports 0.
    seed_studiable_card(&app, teacher_id, "new", 0, 0, 0).await;
    let decks = list_decks(&app, &token).await;
    assert_eq!(decks.len(), 1);
    assert_eq!(decks[0]["options_id"].as_i64(), Some(0), "no preset resolves to 0");

    // Assign a concrete preset to a second deck and assert it is reported.
    let preset_id = common::seed_preset(&app, 1, 8, "suspend_card").await;
    let deck_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by, options_id) VALUES (1, 'Preset Deck', $1, $2) RETURNING id",
        teacher_id,
        preset_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck with preset")
    .id;

    let decks = list_decks(&app, &token).await;
    let assigned = decks.iter().find(|d| d["id"].as_i64() == Some(deck_id)).expect("preset deck listed");
    assert_eq!(
        assigned["options_id"].as_i64(),
        Some(preset_id),
        "assigned preset is reported as the effective id"
    );
}
