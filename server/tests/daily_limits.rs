//! Tests for US-2.6 — per-deck daily-limit overrides (preset / this_deck /
//! today_only).

mod common;

use chrono::{Days, Utc};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use anjuman_server::deck_options::effective_daily_limits;

use common::{UserRole, create_user, seed_studiable_card};

/// Seed a two-deck setup sharing the default preset, and return
/// `(deck_a_id, deck_b_id)` with both attached to nothing special (default
/// preset id 0 limits: new=20, review=200).
async fn seed_two_decks(
    app: &common::TestApp,
    student_id: i64,
) -> (i64, i64) {
    // Two cards in two decks (each card seeds its own deck + preset id 0).
    let deck_a_card = seed_studiable_card(app, student_id, "new", 0, 0, 0).await;
    let deck_b_card = seed_studiable_card(app, student_id, "new", 0, 0, 0).await;

    let deck_a = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", deck_a_card)
        .fetch_one(&app.db)
        .await
        .expect("deck a id");
    let deck_b = sqlx::query_scalar!("SELECT deck_id FROM cards WHERE id = $1", deck_b_card)
        .fetch_one(&app.db)
        .await
        .expect("deck b id");

    (deck_a, deck_b)
}

#[tokio::test]
async fn preset_mode_uses_preset_limit() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (deck_a, _) = seed_two_decks(&app, student_id).await;

    // Default preset id 0: new=20, review=200.
    let (new_per_day, review_per_day) = effective_daily_limits(&app.db, deck_a, Utc::now().date_naive())
        .await
        .expect("resolve limits");

    assert_eq!(new_per_day, 20);
    assert_eq!(review_per_day, 200);
}

#[tokio::test]
async fn this_deck_override_applies_to_one_deck_only() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (deck_a, deck_b) = seed_two_decks(&app, student_id).await;

    // Override deck A's new-per-day to 5.
    sqlx::query!(
        "UPDATE decks SET new_per_day_mode = 'this_deck', new_per_day_override = 5 WHERE id = $1",
        deck_a
    )
    .execute(&app.db)
    .await
    .expect("override deck a");

    let today = Utc::now().date_naive();
    let (a_new, _) = effective_daily_limits(&app.db, deck_a, today).await.expect("a");
    let (b_new, _) = effective_daily_limits(&app.db, deck_b, today).await.expect("b");

    assert_eq!(a_new, 5, "deck A should use its override");
    assert_eq!(b_new, 20, "deck B should still use the preset default");
}

#[tokio::test]
async fn today_only_expires_next_day() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (deck_a, _) = seed_two_decks(&app, student_id).await;

    let today = Utc::now().date_naive();
    sqlx::query!(
        "UPDATE decks SET new_per_day_mode = 'today_only', new_per_day_override = 7, new_per_day_today_date = $1 WHERE id = $2",
        today,
        deck_a
    )
    .execute(&app.db)
    .await
    .expect("set today-only");

    // Today: override applies.
    let (new_today, _) = effective_daily_limits(&app.db, deck_a, today).await.expect("today");
    assert_eq!(new_today, 7);

    // Tomorrow: override expires, falls back to preset (20).
    let tomorrow = today.checked_add_days(Days::new(1)).unwrap();
    let (new_tomorrow, _) = effective_daily_limits(&app.db, deck_a, tomorrow).await.expect("tomorrow");
    assert_eq!(new_tomorrow, 20);
}

#[tokio::test]
async fn new_and_review_limits_are_independent() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (deck_a, _) = seed_two_decks(&app, student_id).await;

    // New limit overridden, review limit left on the preset.
    sqlx::query!(
        "UPDATE decks SET new_per_day_mode = 'this_deck', new_per_day_override = 3 WHERE id = $1",
        deck_a
    )
    .execute(&app.db)
    .await
    .expect("override new only");

    let today = Utc::now().date_naive();
    let (new_per_day, review_per_day) = effective_daily_limits(&app.db, deck_a, today)
        .await
        .expect("resolve");

    assert_eq!(new_per_day, 3, "new limit overridden");
    assert_eq!(review_per_day, 200, "review limit still on the preset");
}

/// Validation: a `this_deck` mode without an override value is rejected with 400.
#[tokio::test]
async fn this_deck_mode_without_value_is_rejected() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_student_id, token) = create_user(&app, UserRole::Teacher).await;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/decks")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    "{\"title\":\"NoLimits\",\"new_per_day_mode\":\"this_deck\"}",
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let _body = res.into_body().collect().await.unwrap().to_bytes();
}
