//! Tests for US-2.7 — subdeck limit aggregation.

mod common;

use anjuman_server::handlers::study::deck_counts_for_student;

use common::{UserRole, create_user};

/// Seed a parent deck with two child decks, each child holding `n_cards` new
/// cards for `student_id`. Returns `(parent_id, child_a_id, child_b_id)`.
async fn seed_parent_with_children(
    app: &common::TestApp,
    student_id: i64,
    n_cards: i64,
) -> (i64, i64, i64) {
    let parent_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, 'Parent', $1) RETURNING id",
        student_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert parent")
    .id;

    let child_a = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by, parent_id) VALUES (1, 'Child A', $1, $2) RETURNING id",
        student_id,
        parent_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert child a")
    .id;

    let child_b = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by, parent_id) VALUES (1, 'Child B', $1, $2) RETURNING id",
        student_id,
        parent_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert child b")
    .id;

    // One note type + template shared for card insertion.
    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names) VALUES (1, $1, '[\"Front\",\"Back\"]') RETURNING id",
        uuid::Uuid::new_v4().to_string()
    )
    .fetch_one(&app.db)
    .await
    .expect("insert note type")
    .id;
    let template_id = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 0, 'Card 1', '{{Front}}', '{{Front}}<hr>{{Back}}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert template")
    .id;

    for deck_id in [child_a, child_b] {
        for _ in 0..n_cards {
            let note_id = sqlx::query!(
                "INSERT INTO notes (note_type_id, fields_json) VALUES ($1, '{\"Front\":\"Q\",\"Back\":\"A\"}') RETURNING id",
                note_type_id
            )
            .fetch_one(&app.db)
            .await
            .expect("insert note")
            .id;
            let card_id = sqlx::query!(
                "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3) RETURNING id",
                note_id,
                deck_id,
                template_id
            )
            .fetch_one(&app.db)
            .await
            .expect("insert card")
            .id;
            sqlx::query!(
                "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0)",
                student_id,
                card_id
            )
            .execute(&app.db)
            .await
            .expect("insert state");
        }
    }

    (parent_id, child_a, child_b)
}

#[tokio::test]
async fn selected_deck_total_caps_the_session() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (parent_id, _child_a, _child_b) = seed_parent_with_children(&app, student_id, 5).await;

    // Two children × 5 new cards = 10 due; parent's default new limit is 20, so
    // all 10 are within budget and the count reflects the physical total.
    let counts = deck_counts_for_student(&app.db, student_id, parent_id)
        .await
        .expect("counts");
    assert_eq!(counts.new_count, 10, "parent default limit (20) admits all 10");
}

#[tokio::test]
async fn subdeck_limit_caps_gathering_from_that_subdeck() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;
    let (parent_id, child_a, child_b) = seed_parent_with_children(&app, student_id, 5).await;

    // Cap child A's new-per-day to 2 (this_deck override); child B stays at 20.
    sqlx::query!(
        "UPDATE decks SET new_per_day_mode = 'this_deck', new_per_day_override = 2 WHERE id = $1",
        child_a
    )
    .execute(&app.db)
    .await
    .expect("cap child a");

    let counts = deck_counts_for_student(&app.db, student_id, parent_id)
        .await
        .expect("counts");

    // 2 (child A, capped) + 5 (child B) = 7 new cards.
    assert_eq!(
        counts.new_count, 7,
        "child A should be capped at 2, child B contributes its full 5 (ids {child_a}/{child_b})"
    );
}
