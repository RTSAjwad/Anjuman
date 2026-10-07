//! Tests for stage 9 (US-9.2) — deck-governance adapters over the relationship
//! model.
//!
//! `check_deck_owner` / `check_deck_collaborator` are now thin adapters over
//! `deck_governance`. These are parity tests: owner/collaborator/admin/stranger
//! (and a missing deck) must produce the same 404/403/Ok as before the refactor.

mod common;

use axum::http::StatusCode;
use anjuman_server::auth::Claims;
use anjuman_server::handlers::decks::{check_deck_collaborator, check_deck_owner};
use common::{UserRole, create_user};

fn claims(user_id: i64, role: UserRole) -> Claims {
    Claims {
        sub: user_id,
        school_id: 1,
        role,
        iat: 0,
        exp: 0,
        jti: String::new(),
    }
}

async fn seed_deck(app: &common::TestApp, owner_id: i64) -> i64 {
    sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, $1, $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        owner_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck")
    .id
}

// ---------------------------------------------------------------------------
// check_deck_owner — owner/admin pass; collaborator/stranger fail.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn check_deck_owner_owner_passes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    check_deck_owner(&app.db, deck_id, 1, &claims(owner_id, UserRole::Teacher))
        .await
        .expect("owner may manage");
}

#[tokio::test]
async fn check_deck_owner_admin_denied_unless_owner() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, owner_id).await;

    // An admin who does not own the deck is denied (same rule as a teacher).
    let err = check_deck_owner(&app.db, deck_id, 1, &claims(admin_id, UserRole::Admin))
        .await
        .expect_err("admin without ownership must be denied");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn check_deck_owner_admin_owner_passes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, admin_id).await;

    check_deck_owner(&app.db, deck_id, 1, &claims(admin_id, UserRole::Admin))
        .await
        .expect("admin owner may manage");
}

#[tokio::test]
async fn check_deck_owner_collaborator_and_stranger_denied() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (collab_id, _ctok) = create_user(&app, UserRole::Teacher).await;
    let (stranger_id, _stok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, NOW())",
        deck_id,
        collab_id
    )
    .execute(&app.db)
    .await
    .expect("add collaborator");

    // A collaborator (not owner) cannot *manage the deck*.
    let err = check_deck_owner(&app.db, deck_id, 1, &claims(collab_id, UserRole::Teacher))
        .await
        .expect_err("collaborator denied ownership");
    assert_eq!(err.0, StatusCode::FORBIDDEN);

    // A stranger is denied.
    let err = check_deck_owner(&app.db, deck_id, 1, &claims(stranger_id, UserRole::Teacher))
        .await
        .expect_err("stranger denied");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn check_deck_owner_missing_deck_is_404() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (user_id, _tok) = create_user(&app, UserRole::Teacher).await;

    let err = check_deck_owner(&app.db, 999_999, 1, &claims(user_id, UserRole::Teacher))
        .await
        .expect_err("missing deck 404");
    assert_eq!(err.0, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// check_deck_collaborator — owner/collaborator/admin pass; stranger fails.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn check_deck_collaborator_owner_passes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    check_deck_collaborator(&app.db, deck_id, 1, &claims(owner_id, UserRole::Teacher))
        .await
        .expect("owner may manage content");
}

#[tokio::test]
async fn check_deck_collaborator_collaborator_passes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (collab_id, _ctok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, NOW())",
        deck_id,
        collab_id
    )
    .execute(&app.db)
    .await
    .expect("add collaborator");

    check_deck_collaborator(&app.db, deck_id, 1, &claims(collab_id, UserRole::Teacher))
        .await
        .expect("collaborator may manage content");
}

#[tokio::test]
async fn check_deck_collaborator_admin_denied_unless_related() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, owner_id).await;

    // An admin who neither owns nor collaborates is denied (same as a teacher).
    let err = check_deck_collaborator(&app.db, deck_id, 1, &claims(admin_id, UserRole::Admin))
        .await
        .expect_err("admin without relationship must be denied");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn check_deck_collaborator_admin_collaborator_passes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, owner_id).await;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, NOW())",
        deck_id,
        admin_id
    )
    .execute(&app.db)
    .await
    .expect("add admin as collaborator");

    check_deck_collaborator(&app.db, deck_id, 1, &claims(admin_id, UserRole::Admin))
        .await
        .expect("admin collaborator may manage content");
}

#[tokio::test]
async fn check_deck_collaborator_stranger_denied() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (stranger_id, _stok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    let err = check_deck_collaborator(&app.db, deck_id, 1, &claims(stranger_id, UserRole::Teacher))
        .await
        .expect_err("stranger denied");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}
