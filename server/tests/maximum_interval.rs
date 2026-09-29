//! Tests for US-2.17 — maximum interval (server-side cap).

mod common;

use common::{UserRole, create_user, seed_preset, seed_studiable_card};

/// The global default preset must default `maximum_interval` to 36500.
#[tokio::test]
async fn maximum_interval_defaults_to_36500() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");
    assert_eq!(options.maximum_interval, 36500);
}

/// A non-default cap set at create time survives a read-back.
#[tokio::test]
async fn maximum_interval_round_trips_through_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("max interval {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({ "name": name, "maximum_interval": 30 });
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
    assert_eq!(options.maximum_interval, 30);
}

/// Values outside 1..=36500 (including the redundant 0) are rejected.
#[tokio::test]
async fn maximum_interval_rejects_out_of_range() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    for bad in [0i64, 36501i64] {
        let body = serde_json::json!({ "name": uuid::Uuid::new_v4().to_string(), "maximum_interval": bad });
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

/// `apply_review` caps the review interval at the preset's maximum, while the
/// stored FSRS `stability` stays uncapped.
#[tokio::test]
async fn apply_review_caps_review_interval_at_preset_maximum() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    // A preset with a 1-day maximum interval attached to a review card with a
    // large stability (so the FSRS interval would naturally exceed 1 day).
    let preset = seed_preset(&app, 1, 8, "suspend_card").await;
    sqlx::query!("UPDATE deck_options SET maximum_interval = 1 WHERE id = $1", preset)
        .execute(&app.db)
        .await
        .expect("set max interval");

    let card_id = seed_studiable_card(&app, student_id, "review", 0, 5, 0).await;
    // Give the card a mature stability so FSRS schedules a multi-day interval.
    sqlx::query!("UPDATE student_card_states SET stability = 90.0 WHERE student_id = $1 AND card_id = $2", student_id, card_id)
        .execute(&app.db)
        .await
        .expect("set stability");
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
        .expect("apply review");

    // The scheduled due interval must be capped at 1 day...
    assert!(
        result.applied_interval_secs <= 86400,
        "review interval must be capped at 1 day, got {}s",
        result.applied_interval_secs
    );
    // ...while the stored FSRS stability reflects the uncapped memory state.
    assert!(
        result.stability > 1.0,
        "stability must stay uncapped, got {}",
        result.stability
    );
}
