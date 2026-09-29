//! Tests for US-2.15 — maximum answer seconds (server-side cap).

mod common;

use common::{UserRole, create_user, seed_preset, seed_studiable_card};

/// The global default preset must default `maximum_answer_seconds` to 60.
#[tokio::test]
async fn maximum_answer_seconds_defaults_to_sixty() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");
    assert_eq!(options.maximum_answer_seconds, 60);
}

/// A non-default cap set at create time survives a read-back.
#[tokio::test]
async fn maximum_answer_seconds_round_trips_through_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("max answer {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({ "name": name, "maximum_answer_seconds": 120 });
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
    assert_eq!(options.maximum_answer_seconds, 120);
}

/// Values outside 1..=7200 are rejected with a Bad Request.
#[tokio::test]
async fn maximum_answer_seconds_rejects_out_of_range() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    for bad in [0i64, 7201i64] {
        let body = serde_json::json!({ "name": uuid::Uuid::new_v4().to_string(), "maximum_answer_seconds": bad });
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
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "value {bad} should be rejected");
    }
}

/// `apply_review` clamps the recorded `response_time_ms` to the preset's cap.
#[tokio::test]
async fn apply_review_caps_response_time_ms_at_preset_maximum() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    // A preset with a low cap (30s) attached to a review card.
    let preset = seed_preset(&app, 1, 8, "suspend_card").await;
    sqlx::query!("UPDATE deck_options SET maximum_answer_seconds = 30 WHERE id = $1", preset)
        .execute(&app.db)
        .await
        .expect("set cap");

    let card_id = seed_studiable_card(&app, student_id, "review", 0, 1, 0).await;
    sqlx::query!(
        "UPDATE decks SET options_id = $1 WHERE id = (SELECT deck_id FROM cards WHERE id = $2)",
        preset,
        card_id
    )
    .execute(&app.db)
    .await
    .expect("attach preset");

    // Send an absurd response time; the stored value must be clamped to 30.
    anjuman_server::handlers::reviews::apply_review(&app.db, student_id, card_id, 3, Some(999_999))
        .await
        .expect("apply review");

    let stored: i64 = sqlx::query_scalar!(
        "SELECT response_time_ms FROM reviews WHERE student_id = $1 AND card_id = $2 ORDER BY id DESC LIMIT 1",
        student_id,
        card_id
    )
    .fetch_one(&app.db)
    .await
    .expect("stored response time")
    .unwrap_or(0);

    assert_eq!(stored, 30, "response time must be capped at the preset maximum");
}
