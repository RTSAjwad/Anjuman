//! Tests for US-2.18a — FSRS parameters (store + consume).

mod common;

use common::{UserRole, create_user, seed_preset, seed_studiable_card};

/// The global default preset must default to empty parameters (= FSRS defaults).
#[tokio::test]
async fn fsrs_parameters_defaults_to_empty() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");
    assert!(options.fsrs_parameters.is_empty());
}

/// A stored parameter vector survives a create → read round-trip.
#[tokio::test]
async fn fsrs_parameters_round_trips_through_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("fsrs params {}", uuid::Uuid::new_v4());
    let params: Vec<f32> = vec![0.1, 1.0, 2.0, 3.0];
    let body = serde_json::json!({ "name": name, "fsrs_parameters": params });
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
    assert_eq!(options.fsrs_parameters, vec![0.1_f32, 1.0, 2.0, 3.0]);
}

/// `apply_review` accepts a preset with stored parameters (consumes them without
/// error) and still produces a positive interval.
#[tokio::test]
async fn apply_review_consumes_stored_parameters() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    let preset = seed_preset(&app, 1, 8, "suspend_card").await;
    // Store a full 21-length weight vector (the FSRS parameter length).
    let params: Vec<f32> = (0..21).map(|i| 0.5 + (i as f32) * 0.01).collect();
    sqlx::query!(
        "UPDATE deck_options SET fsrs_parameters = $1 WHERE id = $2",
        serde_json::to_value(&params).unwrap(),
        preset
    )
    .execute(&app.db)
    .await
    .expect("store params");

    let card_id = seed_studiable_card(&app, student_id, "review", 0, 5, 0).await;
    sqlx::query!(
        "UPDATE decks SET options_id = $1 WHERE id = (SELECT deck_id FROM cards WHERE id = $2)",
        preset,
        card_id
    )
    .execute(&app.db)
    .await
    .expect("attach preset");

    let result = anjuman_server::handlers::reviews::apply_review(&app.db, student_id, card_id, 3, None)
        .await
        .expect("apply review with stored params");
    assert!(result.applied_interval_secs > 0, "stored params must schedule a positive interval");
}
