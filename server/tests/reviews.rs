//! Tests for US-2.1 (Hard-button) and US-2.4 (empty relearning steps).
//!
//! These exercise `apply_review` (the scheduling authority) directly, which is
//! the observable surface for the Hard-button and skip-relearning rules, seeded
//! with the default preset's learning steps (`1m 10m`) → average of `[60, 600]`
//! = 330s.

mod common;

use anjuman_server::handlers::reviews::apply_review;

use common::{
    UserRole, create_user, seed_empty_relearning_preset, seed_studiable_card,
};

/// Hard on a *new* card (first learning step) gives the average of the first
/// two steps (330s for the default `1m 10m`).
#[tokio::test]
async fn hard_on_new_card_uses_average_of_first_two_steps() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let card_id = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;

    let reviewed = apply_review(&app.db, student_id, card_id, 2, None)
        .await
        .expect("apply review");

    assert_eq!(reviewed.state, "learning");
    assert_eq!(reviewed.applied_interval_secs, 330);
    // Hard does not advance the step index on a new card (stays at 0).
    assert_eq!(reviewed.step_index, 0);
}

/// Hard on a *learning* card in a later step repeats the current step's delay
/// (does not advance).
#[tokio::test]
async fn hard_on_later_learning_step_repeats_current_step() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    // Seed a learning card already on step 1 (the `10m` step).
    let card_id = seed_studiable_card(&app, student_id, "learning", 1, 1, 0).await;

    let reviewed = apply_review(&app.db, student_id, card_id, 2, None)
        .await
        .expect("apply review");

    assert_eq!(reviewed.state, "learning");
    assert_eq!(reviewed.applied_interval_secs, 600); // 10m, unchanged
    assert_eq!(reviewed.step_index, 1); // not advanced
}

/// US-2.4 — a review card with empty relearning steps skips relearning on
/// "Again" and has its interval recomputed directly by FSRS.
#[tokio::test]
async fn empty_relearning_steps_skip_relearning_on_lapse() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    let preset = seed_empty_relearning_preset(&app, 1).await;
    let card_id = seed_studiable_card(&app, student_id, "review", 0, 1, 0).await;

    // Attach the empty-relearning preset to the card's deck.
    sqlx::query!(
        "UPDATE decks SET options_id = $1 WHERE id = (SELECT deck_id FROM cards WHERE id = $2)",
        preset,
        card_id
    )
    .execute(&app.db)
    .await
    .expect("attach preset");

    let reviewed = apply_review(&app.db, student_id, card_id, 1, None)
        .await
        .expect("apply review");

    // Must stay review (no relearning phase) and get a non-zero FSRS interval.
    assert_eq!(reviewed.state, "review");
    assert_eq!(reviewed.step_index, 0);
    assert!(reviewed.applied_interval_secs > 0);
}
