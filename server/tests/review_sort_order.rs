//! Tests for US-2.12 — review sort order (enum plumbing, defaults, ordering).

mod common;

use common::{UserRole, create_user};

use anjuman_server::deck_options::get_options;

/// Seed a deck with `n` review cards, each with the given `stability` (days),
/// all due at `due_at`. Returns `(deck_id, Vec<card_id>)` in creation order
/// (which is also ascending `stability` order, matching the argument order).
async fn seed_review_cards(
    app: &common::TestApp,
    student_id: i64,
    stabilities: &[f64],
    due_at: chrono::DateTime<chrono::Utc>,
) -> (i64, Vec<i64>) {
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

    let mut card_ids = Vec::new();
    for stability in stabilities {
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
            stability,
            due_at
        )
        .execute(&app.db)
        .await
        .expect("insert review state");

        card_ids.push(card_id);
    }

    (deck_id, card_ids)
}

/// The default preset (id 0) must default to `due_then_random`.
#[tokio::test]
async fn review_sort_order_defaults_to_due_then_random() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_student_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = get_options(&app.db, 0).await.expect("default preset");
    assert_eq!(
        options.review_sort_order,
        anjuman_contracts::deck_options::ReviewSortOrder::DueThenRandom
    );
}

/// A non-default review sort order round-trips through the create handler.
#[tokio::test]
async fn review_sort_order_round_trips_through_the_handler() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("Review sort {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({ "name": name, "review_sort_order": "ascending_interval" });
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/deck-options")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let res_body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&res_body).unwrap();

    assert_eq!(json["review_sort_order"], "ascending_interval");
    let id = json["id"].as_i64().unwrap();

    let options = get_options(&app.db, id).await.expect("read back");
    assert_eq!(
        options.review_sort_order,
        anjuman_contracts::deck_options::ReviewSortOrder::AscendingInterval
    );
}

/// Updating a preset's `review_sort_order` persists the new value.
#[tokio::test]
async fn review_sort_order_can_be_updated() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    // Create with the default, then PATCH to a non-default value.
    let create_name = format!("Update sort {}", uuid::Uuid::new_v4());
    let create = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/deck-options")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "name": create_name }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_body = create.into_body().collect().await.unwrap().to_bytes();
    let id = serde_json::from_slice::<serde_json::Value>(&create_body).unwrap()["id"]
        .as_i64()
        .unwrap();

    let patch = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/deck-options/{id}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "review_sort_order": "relative_overdueness" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch.status(), StatusCode::OK);

    let options = get_options(&app.db, id).await.expect("read back");
    assert_eq!(
        options.review_sort_order,
        anjuman_contracts::deck_options::ReviewSortOrder::RelativeOverdueness
    );
}

/// Ascending-interval review sort returns the lowest-stability review card
/// first, regardless of creation (id) order.
#[tokio::test]
async fn ascending_interval_orders_review_cards_by_stability() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, token) = create_user(&app, UserRole::Student).await;

    // Seed three review cards with stabilities out of order, all due in the
    // past (so they are due now) and therefore studyable.
    let past = chrono::Utc::now() - chrono::Duration::days(1);
    let (deck_id, card_ids) =
        seed_review_cards(&app, student_id, &[10.0, 2.0, 5.0], past).await;
    // card_ids[1] has stability 2.0 (the lowest), card_ids[0] has 10.0.

    // Attach an ascending_interval preset to the deck.
    let preset = sqlx::query!(
        "INSERT INTO deck_options (school_id, name, review_sort_order) VALUES (1, $1, $2::text::review_sort_order) RETURNING id",
        format!("ascending interval {}", uuid::Uuid::new_v4()),
        "ascending_interval"
    )
    .fetch_one(&app.db)
    .await
    .expect("insert preset")
    .id;
    sqlx::query!("UPDATE decks SET options_id = $1 WHERE id = $2", preset, deck_id)
        .execute(&app.db)
        .await
        .expect("attach preset");

    // Study the deck; the first due card must be the lowest-stability card.
    use axum::body::Body;
    use axum::http::Request;
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

    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let next_card_id = json["next_card"]["card_id"].as_i64().expect("next card");
    assert_eq!(next_card_id, card_ids[1], "lowest-stability card must be first");
}
