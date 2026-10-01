//! Tests for US-4.7 — centralized deck-access permission predicate.
//!
//! Verifies deck permissions are *resource-grants*, not role-gates: study is
//! granted by ownership / collaboration / class membership (over the subtree),
//! and an `Admin` role alone is **not** a blanket grant.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use anjuman_server::auth::Claims;
use anjuman_server::permissions::{DeckPerm, deck_permission, require_deck_perm};
use common::{UserRole, create_user};

fn get(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

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

/// A deck owned by `owner_id`. Returns its id.
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

/// Study is a resource grant: an owner gets `DeckPerm::Study`, a non-owner admin
/// does **not** — for the same deck.
#[tokio::test]
async fn admin_is_not_blanket_granted_study() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;

    // A deck owned by the teacher (the admin has no relationship to it).
    let deck_id = seed_deck(&app, teacher_id).await;

    // Owner (teacher) → Study.
    let perm = deck_permission(&app.db, &claims(teacher_id, UserRole::Teacher), deck_id)
        .await
        .expect("perm");
    assert_eq!(perm, DeckPerm::Study);

    // Admin with no owner/collaborator/class relationship → not Study.
    let admin_perm = deck_permission(&app.db, &claims(admin_id, UserRole::Admin), deck_id)
        .await
        .expect("perm");
    assert_eq!(admin_perm, DeckPerm::None);
}

/// study `GET /decks/{id}/study` is rejected for an admin who does not own or
/// collaborate on the deck (the US-4.7 behaviour change).
#[tokio::test]
async fn admin_study_rejected_on_deck_they_dont_own() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (_admin_id, atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{deck_id}/study"), &atok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "admin without a grant must be 403");
}

/// An admin who *owns* a deck can study it (grant by ownership, not role).
#[tokio::test]
async fn admin_owner_can_study() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (admin_id, atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, admin_id).await;

    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{deck_id}/study"), &atok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "admin-owner should be studyable");
}

/// A collaborator (any role) can study; collaboration is a grant independent of
/// role.
#[tokio::test]
async fn collaborator_gets_study_grant() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (other_id, otok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, NOW())",
        deck_id,
        other_id
    )
    .execute(&app.db)
    .await
    .expect("add collaborator");

    let perm = deck_permission(&app.db, &claims(other_id, UserRole::Teacher), deck_id)
        .await
        .expect("perm");
    assert_eq!(perm, DeckPerm::Study);

    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{deck_id}/study"), &otok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// `require_deck_perm` admits Study for a granted deck and denies otherwise.
#[tokio::test]
async fn require_deck_perm_enforces_study() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    // Owner passes the Study requirement.
    require_deck_perm(&app.db, &claims(teacher_id, UserRole::Teacher), deck_id, DeckPerm::Study)
        .await
        .expect("owner holds Study");

    // Non-granted admin is denied.
    let err = require_deck_perm(&app.db, &claims(admin_id, UserRole::Admin), deck_id, DeckPerm::Study)
        .await
        .expect_err("admin denied Study");
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}
