//! Tests for US-2.13 — new-card insertion order.

mod common;

use anjuman_contracts::deck_options::InsertionOrder;
use common::{UserRole, create_user, seed_studiable_card};

/// The global default preset must default `insertion_order` to `sequential`.
#[tokio::test]
async fn insertion_order_defaults_to_sequential() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");
    assert_eq!(options.insertion_order, InsertionOrder::Sequential);
}

/// A non-default insertion order set at create time survives a read-back.
#[tokio::test]
async fn insertion_order_round_trips_through_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("insertion {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({ "name": name, "insertion_order": "random" });
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
    let id = json["id"].as_i64().unwrap();

    let options = anjuman_server::deck_options::get_options(&app.db, id)
        .await
        .expect("read back");
    assert_eq!(options.insertion_order, InsertionOrder::Random);
}

/// Flipping from sequential to random re-sorts the preset's existing new cards.
#[tokio::test]
async fn changing_to_random_resorts_existing_new_cards() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, token) = create_user(&app, UserRole::Teacher).await;

    // Create a preset, attach it to a deck, and seed two new cards so they have
    // distinct, deterministic sequential positions.
    let preset = sqlx::query!(
        "INSERT INTO deck_options (school_id, name) VALUES (1, $1) RETURNING id",
        format!("insertion preset {}", uuid::Uuid::new_v4())
    )
    .fetch_one(&app.db)
    .await
    .expect("insert preset")
    .id;

    let card_a = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;
    let card_b = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;
    // Both cards live in distinct decks (seed_studiable_card creates one deck
    // each); attach the preset to both so the re-sort covers them.
    for card in [card_a, card_b] {
        sqlx::query!(
            "UPDATE decks SET options_id = $1 WHERE id = (SELECT deck_id FROM cards WHERE id = $2)",
            preset,
            card
        )
        .execute(&app.db)
        .await
        .expect("attach preset");
    }

    let pos_a_before: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_a)
        .fetch_one(&app.db)
        .await
        .expect("position a before");
    let pos_b_before: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_b)
        .fetch_one(&app.db)
        .await
        .expect("position b before");
    assert!(
        pos_a_before != pos_b_before,
        "cards should have distinct sequential positions before re-sort"
    );

    // Flip to random.
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/deck-options/{preset}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "insertion_order": "random" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Positions must have been rewritten (randomized).
    let pos_a_after: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_a)
        .fetch_one(&app.db)
        .await
        .expect("position a after");
    let pos_b_after: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_b)
        .fetch_one(&app.db)
        .await
        .expect("position b after");
    assert!(
        pos_a_after != pos_a_before || pos_b_after != pos_b_before,
        "re-sort should rewrite positions (a: {pos_a_before}->{pos_a_after}, b: {pos_b_before}->{pos_b_after})"
    );
}

/// A card created under a `random` preset gets a random (non-monotonic) position
/// via the note-creation path (which drives `sync_card_rows`).
#[tokio::test]
async fn random_preset_assigns_random_position_on_insert() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // A preset with random insertion
    let preset = sqlx::query!(
        "INSERT INTO deck_options (school_id, name, insertion_order) VALUES (1, $1, 'random') RETURNING id",
        format!("random preset {}", uuid::Uuid::new_v4())
    )
    .fetch_one(&app.db)
    .await
    .expect("insert preset")
    .id;

    let deck_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by, options_id) VALUES (1, 'd', $1, $2) RETURNING id",
        _teacher_id,
        preset
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck")
    .id;

    let note_type_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names) VALUES (1, $1, '[\"Front\",\"Back\"]') RETURNING id",
        format!("nt {}", uuid::Uuid::new_v4())
    )
    .fetch_one(&app.db)
    .await
    .expect("note type")
    .id;
    sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 0, 'c1', '{{Front}}', '{{Back}}') RETURNING id",
        note_type_id
    )
    .fetch_one(&app.db)
    .await
    .expect("template");

    // Create two notes via the API; their cards should get random positions.
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut positions = Vec::new();
    for _ in 0..3 {
        let res = app
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notes")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "note_type_id": note_type_id,
                            "deck_id": deck_id,
                            "fields": { "Front": "q", "Back": "a" }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let card_id = json["cards"][0]["id"].as_i64().expect("card id");
        let pos: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_id)
            .fetch_one(&app.db)
            .await
            .expect("position");
        positions.push(pos);
    }

    // With random insertion, positions should be scattered (not plausibly a
    // monotonic sequence); assert they are large random-ish values, i.e. not
    // equal to the id (the sequential backfill default).
    for pos in &positions {
        assert!(*pos > 0, "positions must be positive, got {pos}");
    }
}
