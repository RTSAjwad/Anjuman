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

/// The global default preset (id 0, owned by the system school) is fetchable
/// from any school (US-4.9 carve-out): `GET /deck-options/0` must not 404 for a
/// real-school caller, since a deck with no `options_id` resolves to it.
#[tokio::test]
async fn get_deck_options_resolves_global_default_for_other_school() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    let res = app
        .app
        .clone()
        .oneshot(get("/deck-options/0", &token))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "id 0 resolves across schools");

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"].as_i64(), Some(0));
    assert_eq!(json["name"].as_str(), Some("Default"));
}

/// The global default preset (id 0) is read-only for real-school users: `PATCH`
/// and `DELETE` on it are rejected (403), so no school can mutate/delete the
/// shared system default — while their *own* presets remain fully editable.
#[tokio::test]
async fn global_default_preset_is_read_only_for_other_schools() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // PATCH /deck-options/0 -> 403.
    let patch = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/deck-options/0")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "new_per_day": 99 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch.status(), StatusCode::FORBIDDEN, "PATCH id 0 is forbidden");

    // DELETE /deck-options/0 -> 403.
    let delete = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/deck-options/0")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::FORBIDDEN, "DELETE id 0 is forbidden");

    // The default preset survives untouched.
    let still_there = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("id 0 still present");
    assert_eq!(still_there.name, "Default");
}
