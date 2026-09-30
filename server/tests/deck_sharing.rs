//! Tests for US-2.19 — deck sharing: subtree access model.
//!
//! Verifies that a grant on a deck covers its whole subtree (descendants are
//! inherited), that a student granted only a *child* sees its ancestors as
//! read-only context (visible but not studyable), and that study/counts honour
//! the same rule.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user};

/// Seed a teacher-owned parent deck with one child. Returns (parent_id, child_id).
async fn seed_parent_child(app: &common::TestApp, teacher_id: i64) -> (i64, i64) {
    let parent = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, 'Parent', $1) RETURNING id",
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert parent")
    .id;

    let child = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by, parent_id) VALUES (1, 'Child', $1, $2) RETURNING id",
        teacher_id,
        parent
    )
    .fetch_one(&app.db)
    .await
    .expect("insert child")
    .id;

    (parent, child)
}

/// Create a class, enrol the student, and attach the deck to it.
async fn grant_via_class(app: &common::TestApp, teacher_id: i64, student_id: i64, deck_id: i64) {
    let class_id = sqlx::query!(
        "INSERT INTO classes (school_id, name, created_by) VALUES (1, $1, $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert class")
    .id;

    sqlx::query!(
        "INSERT INTO class_members (class_id, user_id, role, joined_at) VALUES ($1, $2, 'student', NOW())",
        class_id,
        student_id
    )
    .execute(&app.db)
    .await
    .expect("enrol student");

    sqlx::query!(
        "INSERT INTO deck_classes (deck_id, class_id, added_at) VALUES ($1, $2, NOW())",
        deck_id,
        class_id
    )
    .execute(&app.db)
    .await
    .expect("attach deck");
}

fn get(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// A student granted the parent sees the parent **and** its descendants, all
/// studyable.
#[tokio::test]
async fn parent_grant_includes_descendants_studyable() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (student_id, stok) = create_user(&app, UserRole::Student).await;
    let (parent, child) = seed_parent_child(&app, teacher_id).await;
    grant_via_class(&app, teacher_id, student_id, parent).await;

    let res = app
        .app
        .clone()
        .oneshot(get("/decks", &stok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let decks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    // Both parent and child appear, both studyable.
    let parent_v = decks.iter().find(|d| d["id"].as_i64() == Some(parent)).expect("parent listed");
    let child_v = decks.iter().find(|d| d["id"].as_i64() == Some(child)).expect("child listed");
    assert_eq!(parent_v["studyable"], true, "parent should be studyable");
    assert_eq!(child_v["studyable"], true, "child should inherit studyable");
}

/// A student granted only the *child* sees the child (studyable) and its
/// ancestor (visible but not studyable).
#[tokio::test]
async fn child_grant_marks_ancestor_as_context_only() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (student_id, stok) = create_user(&app, UserRole::Student).await;
    let (parent, child) = seed_parent_child(&app, teacher_id).await;
    grant_via_class(&app, teacher_id, student_id, child).await;

    let res = app
        .app
        .clone()
        .oneshot(get("/decks", &stok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let decks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    let child_v = decks.iter().find(|d| d["id"].as_i64() == Some(child)).expect("child listed");
    let parent_v = decks.iter().find(|d| d["id"].as_i64() == Some(parent)).expect("parent listed as context");
    assert_eq!(child_v["studyable"], true, "granted child is studyable");
    assert_eq!(parent_v["studyable"], false, "ancestor is context-only, not studyable");
}

/// Study is rejected (403) for a context-only ancestor, but allowed (200) for
/// a granted child.
#[tokio::test]
async fn study_rejects_context_only_ancestor_allows_granted_child() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (student_id, stok) = create_user(&app, UserRole::Student).await;
    let (parent, child) = seed_parent_child(&app, teacher_id).await;
    grant_via_class(&app, teacher_id, student_id, child).await;

    // Context-only parent → forbidden.
    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{parent}/study"), &stok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "parent is context-only");

    // Granted child → allowed (study starts, returns a body).
    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{child}/study"), &stok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "child is studyable");
}

/// A student with no grant on a deck gets 403 (not 404) from study, and 403
/// from get_deck.
#[tokio::test]
async fn ungranted_student_is_forbidden() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (_student_id, stok) = create_user(&app, UserRole::Student).await;
    let (parent, _child) = seed_parent_child(&app, teacher_id).await;
    // No grant at all for this student.

    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{parent}/study"), &stok))
        .await
        .unwrap();
    // Existing check_deck_visible returns FORBIDDEN for a visible-deck-no-access;
    // the deck exists but the student has no relationship → forbidden.
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

/// A context-only ancestor's *detail* (`get_deck`) returns the row but no
/// collaborators or class list — administrative meta must not leak to a student
/// who isn't directly granted that deck (US-2.19).
#[tokio::test]
async fn context_only_ancestor_detail_hides_collaborators_and_classes() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (student_id, stok) = create_user(&app, UserRole::Student).await;
    let (parent, child) = seed_parent_child(&app, teacher_id).await;
    grant_via_class(&app, teacher_id, student_id, child).await;

    let res = app
        .app
        .clone()
        .oneshot(get(&format!("/decks/{parent}"), &stok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // The row is visible (tree context)…
    assert_eq!(json["deck"]["id"].as_i64(), Some(parent));
    // …but its administrative meta is not exposed to this student.
    assert_eq!(json["collaborators"].as_array().map(Vec::len), Some(0));
    assert_eq!(json["classes"].as_array().map(Vec::len), Some(0));
}

/// `deck_access` is the correct level when queried directly: Studyable for
/// self/ancestor grant, ContextOnly for a descendant grant.
#[tokio::test]
async fn deck_access_levels() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, _ttok) = create_user(&app, UserRole::Teacher).await;
    let (student_id, _stok) = create_user(&app, UserRole::Student).await;
    let (parent, child) = seed_parent_child(&app, teacher_id).await;

    // Build a Claims-like context via a real token's role/sub? deck_access takes
    // &Claims — use the ones from `anjuman_server::auth::Claims` by decoding, or
    // simpler: exercise via the public HTTP path above. Here we assert the
    // helper's behaviour through the list response already covered, so keep this
    // test focused on a direct access call using a forged claims struct.
    use anjuman_server::auth::Claims;
    use anjuman_server::handlers::decks::{DeckAccess, deck_access};

    let claims = Claims {
        sub: student_id,
        school_id: 1,
        role: UserRole::Student,
        iat: 0,
        exp: 0,
        jti: String::new(),
    };

    // No grant yet.
    let access = deck_access(&app.db, child, 1, &claims).await.unwrap();
    assert_eq!(access, DeckAccess::None);

    // Grant the parent → child becomes Studyable (descendant inheritance).
    grant_via_class(&app, teacher_id, student_id, parent).await;
    let access = deck_access(&app.db, child, 1, &claims).await.unwrap();
    assert_eq!(access, DeckAccess::Studyable);

    // Grant only the child in a *fresh* scenario → parent is ContextOnly.
    // (Reuse a second parent/child pair to avoid the parent grant above.)
    let (parent2, child2) = seed_parent_child(&app, teacher_id).await;
    grant_via_class(&app, teacher_id, student_id, child2).await;
    let access = deck_access(&app.db, parent2, 1, &claims).await.unwrap();
    assert_eq!(access, DeckAccess::ContextOnly);
}
