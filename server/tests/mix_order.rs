//! Tests for the `mix` display order (US-2.10/US-2.11) — Anki-faithful
//! `Intersperser` interleave, restored statelessly.

mod common;

use common::{UserRole, create_user};

/// Seed a deck with `n_reviews` review cards (all due now, distinct stability)
/// and `n_new` new cards, attached to a preset with the given `new_review_order`.
/// Returns `(deck_id, review_ids, new_ids)`.
async fn seed_mixed_deck(
    app: &common::TestApp,
    student_id: i64,
    n_reviews: usize,
    n_new: usize,
    new_review_order: &str,
) -> (i64, Vec<i64>, Vec<i64>) {
    let deck_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, $1, $2) RETURNING id",
        uuid::Uuid::new_v4().to_string(),
        student_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck")
    .id;

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

    let past = chrono::Utc::now() - chrono::Duration::days(1);

    let mut review_ids = Vec::new();
    for i in 0..n_reviews {
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
            "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses, step_index, due_at) VALUES ($1, $2, 'review', $3, 0.0, 1, 0, 0, $4)",
            student_id,
            card_id,
            (i as f64) + 1.0,
            past
        )
        .execute(&app.db)
        .await
        .expect("insert review state");
        review_ids.push(card_id);
    }

    let mut new_ids = Vec::new();
    for _ in 0..n_new {
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
            "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses, step_index) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0, 0)",
            student_id,
            card_id
        )
        .execute(&app.db)
        .await
        .expect("insert new state");
        new_ids.push(card_id);
    }

    // Attach a preset with the requested new/review order.
    let preset = sqlx::query!(
        "INSERT INTO deck_options (school_id, name, new_review_order) VALUES (1, $1, $2::text::new_review_order) RETURNING id",
        format!("mix preset {}", uuid::Uuid::new_v4()),
        new_review_order
    )
    .fetch_one(&app.db)
    .await
    .expect("insert preset")
    .id;
    sqlx::query!("UPDATE decks SET options_id = $1 WHERE id = $2", preset, deck_id)
        .execute(&app.db)
        .await
        .expect("attach preset");

    (deck_id, review_ids, new_ids)
}

/// Fetch the study stream's card ids (in presentation order) by repeatedly
/// calling the study endpoint until `next_card` is null, answering each card
/// with Good so it doesn't immediately re-queue.
async fn drain_study_stream(
    app: &common::TestApp,
    token: &str,
    deck_id: i64,
) -> Vec<i64> {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut order = Vec::new();
    // Study up to a bounded number of cards to avoid infinite loops on
    // seed-data interactions.
    for _ in 0..200 {
        let res = app
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/decks/{deck_id}/study"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let card = match json["next_card"].as_object() {
            Some(c) => c,
            None => break,
        };
        let card_id = card["card_id"].as_i64().expect("card id");
        order.push(card_id);

        // Answer Good to advance.
        let advance = app
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/decks/{deck_id}/study"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "card_id": card_id, "rating": 3 }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(advance.status(), StatusCode::OK);
    }
    order
}

/// With `new_review_order = mix` and an equal number of new and review cards,
/// the stream must interleave them (Anki `Intersperser`: a,b,a,b,…).
#[tokio::test]
async fn mix_interleaves_new_and_review_cards() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, token) = create_user(&app, UserRole::Student).await;

    let (deck_id, review_ids, new_ids) = seed_mixed_deck(&app, student_id, 2, 2, "mix").await;

    let order = drain_study_stream(&app, &token, deck_id).await;

    // The first four cards (the mixed set) must alternate review/new (or
    // new/review), i.e. the two classes must be interspersed, not block-ordered.
    let first = &order[..4.min(order.len())];
    let review_first_two: Vec<i64> = review_ids.iter().copied().take(2).collect();

    // After draining, each of the 2 review + 2 new cards has been served once in
    // the studied stream (Good on a review card re-schedules it to a future day,
    // so it won't reappear in this session; Good on a new card moves it to
    // learning/review).
    for id in review_ids.iter().chain(new_ids.iter()) {
        assert!(order.contains(id), "card {id} not served");
    }

    // At least one review and one new card appear among the first two served,
    // proving interleaving (not "all reviews then all new").
    let first_classes: Vec<bool> = first
        .iter()
        .map(|id| review_first_two.contains(id))
        .collect();
    assert!(
        first_classes.iter().any(|&is_review| is_review),
        "expected a review card early in the mixed stream: {first:?}"
    );
    assert!(
        first_classes.iter().any(|&is_review| !is_review),
        "expected a new card early in the mixed stream: {first:?}"
    );
}

/// `new_review_order = after` must NOT interleave: all reviews before new.
#[tokio::test]
async fn after_orders_reviews_before_new() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, token) = create_user(&app, UserRole::Student).await;

    let (deck_id, review_ids, new_ids) = seed_mixed_deck(&app, student_id, 2, 2, "after").await;

    // Only assert the very first card is a review (the block guarantee), since
    // draining fully would re-schedule cards.
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/decks/{deck_id}/study"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let first_card = json["next_card"]["card_id"].as_i64().expect("first card");
    assert!(
        review_ids.contains(&first_card),
        "first card with `after` must be a review, got {first_card}"
    );
    assert!(!new_ids.contains(&first_card));
}
