//! Tests for stage 9 (US-9.1) — consolidated capability predicates + deck
//! governance levels.
//!
//! Verifies the two-step model: the role-bundle capability predicates are pure
//! `Claims → bool`, and `deck_governance` returns the governance levels
//! (`Manage`/`Contribute`) only when *both* the role bundle admits governance
//! *and* the relationship holds — not on relationship alone, and governance is
//! orthogonal to study.

mod common;

use anjuman_server::auth::Claims;
use anjuman_server::permissions::{
    DeckGovernance, can_administer_school, can_manage_classes, can_manage_decks, can_manage_notes,
    can_manage_note_types, can_manage_presets, can_manage_tags, can_manage_users, deck_governance,
};
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
// Role-bundle capability predicates (pure `Claims → bool`)
// ---------------------------------------------------------------------------

#[test]
fn role_bundle_teacher_can_manage_decks() {
    assert!(can_manage_decks(&claims(1, UserRole::Teacher)));
    assert!(can_manage_notes(&claims(1, UserRole::Teacher)));
    assert!(can_manage_presets(&claims(1, UserRole::Teacher)));
    assert!(can_manage_note_types(&claims(1, UserRole::Teacher)));
    assert!(can_manage_tags(&claims(1, UserRole::Teacher)));
    assert!(can_manage_classes(&claims(1, UserRole::Teacher)));
}

#[test]
fn role_bundle_student_cannot_manage_decks() {
    assert!(!can_manage_decks(&claims(1, UserRole::Student)));
    assert!(!can_manage_notes(&claims(1, UserRole::Student)));
    assert!(!can_manage_presets(&claims(1, UserRole::Student)));
    assert!(!can_manage_note_types(&claims(1, UserRole::Student)));
    assert!(!can_manage_tags(&claims(1, UserRole::Student)));
    assert!(!can_manage_classes(&claims(1, UserRole::Student)));
}

#[test]
fn role_bundle_admin_can_administer_and_manage_users() {
    assert!(can_administer_school(&claims(1, UserRole::Admin)));
    assert!(can_manage_users(&claims(1, UserRole::Admin)));
    assert!(!can_administer_school(&claims(1, UserRole::Teacher)));
    assert!(!can_manage_users(&claims(1, UserRole::Teacher)));
}

// ---------------------------------------------------------------------------
// Deck governance levels (relationship gated by the role bundle)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deck_governance_returns_manage_for_owner() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, teacher_id).await;

    let gov = deck_governance(&app.db, &claims(teacher_id, UserRole::Teacher), deck_id)
        .await
        .expect("governance");
    assert_eq!(gov, DeckGovernance::Manage);
}

#[tokio::test]
async fn deck_governance_returns_contribute_for_collaborator() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (other_id, _otok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, NOW())",
        deck_id,
        other_id
    )
    .execute(&app.db)
    .await
    .expect("add collaborator");

    let gov = deck_governance(&app.db, &claims(other_id, UserRole::Teacher), deck_id)
        .await
        .expect("governance");
    assert_eq!(gov, DeckGovernance::Contribute);
}

#[tokio::test]
async fn deck_governance_returns_manage_for_admin() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (admin_id, _atok) = create_user(&app, UserRole::Admin).await;
    let deck_id = seed_deck(&app, owner_id).await;

    // Admin who does not own the deck still gets `Manage` (admin trump).
    let gov = deck_governance(&app.db, &claims(admin_id, UserRole::Admin), deck_id)
        .await
        .expect("governance");
    assert_eq!(gov, DeckGovernance::Manage);
}

#[tokio::test]
async fn deck_governance_returns_none_for_unrelated() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (owner_id, _tok) = create_user(&app, UserRole::Teacher).await;
    let (unrelated_id, _utok) = create_user(&app, UserRole::Teacher).await;
    let deck_id = seed_deck(&app, owner_id).await;

    let gov = deck_governance(&app.db, &claims(unrelated_id, UserRole::Teacher), deck_id)
        .await
        .expect("governance");
    assert_eq!(gov, DeckGovernance::None);
}

/// A *student* owner/collaborator gets no governance — governance is role-gated
/// (a student's relationship is a study grant only).
#[tokio::test]
async fn student_owner_does_not_get_governance() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _stok) = create_user(&app, UserRole::Student).await;
    let deck_id = seed_deck(&app, student_id).await;

    let gov = deck_governance(&app.db, &claims(student_id, UserRole::Student), deck_id)
        .await
        .expect("governance");
    assert_eq!(gov, DeckGovernance::None);
}
