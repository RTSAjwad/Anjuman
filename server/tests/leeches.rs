//! Tests for US-2.2 (leech threshold) and US-2.3 (leech action).

mod common;

use anjuman_server::handlers::reviews::apply_review;

use common::{UserRole, create_user, seed_preset, seed_studiable_card};

/// Attach `preset_id` to the deck that owns `card_id`, so the next review uses
/// its leech config instead of the global default preset.
async fn attach_preset(app: &common::TestApp, card_id: i64, preset_id: i64) {
    sqlx::query!(
        "UPDATE decks SET options_id = $1 WHERE id = (SELECT deck_id FROM cards WHERE id = $2)",
        preset_id,
        card_id
    )
    .execute(&app.db)
    .await
    .expect("attach preset to deck");
}

/// US-2.2 — a review-card "Again" increments `lapses`; a learning "Again" does
/// not. Here we assert the persisted counter semantics directly.
#[tokio::test]
async fn lapse_counts_review_card_again_only() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    // A learning card answered "Again" must NOT increment the leech counter.
    let learning_card = seed_studiable_card(&app, student_id, "learning", 0, 1, 0).await;
    let _ = apply_review(&app.db, student_id, learning_card, 1, None)
        .await
        .expect("apply review");
    let learning_lapses = sqlx::query_scalar!(
        "SELECT lapses FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        student_id,
        learning_card
    )
    .fetch_one(&app.db)
    .await
    .expect("read lapses");

    assert_eq!(learning_lapses, 0, "learning 'Again' must not count as a lapse");

    // A review card answered "Again" must increment the counter.
    let review_card = seed_studiable_card(&app, student_id, "review", 0, 1, 0).await;
    let _ = apply_review(&app.db, student_id, review_card, 1, None)
        .await
        .expect("apply review");
    let review_lapses = sqlx::query_scalar!(
        "SELECT lapses FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        student_id,
        review_card
    )
    .fetch_one(&app.db)
    .await
    .expect("read lapses");

    assert_eq!(review_lapses, 1, "review 'Again' must increment the lapse count");
}

/// US-2.3 — `SuspendCard` suspends the card once the threshold is reached.
#[tokio::test]
async fn suspend_card_action_suspends_at_threshold() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    let preset = seed_preset(&app, 1, 1, "suspend_card").await;
    let card_id = seed_studiable_card(&app, student_id, "review", 0, 1, 0).await;
    attach_preset(&app, card_id, preset).await;

    let _ = apply_review(&app.db, student_id, card_id, 1, None)
        .await
        .expect("apply review");

    let suspended = sqlx::query_scalar!(
        "SELECT suspended FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        student_id,
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("read suspended");

    assert!(suspended, "leech card should be suspended at threshold");
}

/// US-2.3 — `TagOnly` does not suspend (a documented no-op until tags land).
#[tokio::test]
async fn tag_only_action_does_not_suspend() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    let preset = seed_preset(&app, 1, 1, "tag_only").await;
    let card_id = seed_studiable_card(&app, student_id, "review", 0, 1, 0).await;
    attach_preset(&app, card_id, preset).await;

    let _ = apply_review(&app.db, student_id, card_id, 1, None)
        .await
        .expect("apply review");

    let suspended = sqlx::query_scalar!(
        "SELECT suspended FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        student_id,
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("read suspended");

    assert!(!suspended, "TagOnly must not suspend the card");

    // The note should still be marked as a leech (leech_tagged_at set).
    let marked = sqlx::query_scalar!(
        "SELECT leech_tagged_at IS NOT NULL FROM notes WHERE id = (SELECT note_id FROM cards WHERE id = $1)",
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("read leech marker")
    .unwrap_or(false);

    assert!(marked, "note should be marked as a leech at threshold");
}
