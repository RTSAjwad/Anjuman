//! Tests for stage 3 — preferences CRUD (`GET/PATCH /preferences`).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user};

/// Build a `PATCH /preferences` request with the given token and JSON body.
fn patch(token: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri("/preferences")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Reading preferences before any update returns the Anki defaults, without
/// the server having persisted a row.
#[tokio::test]
async fn preferences_default_to_anki_values_before_any_update() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (user_id, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["learn_ahead_minutes"], 20);
    assert_eq!(json["day_start_hour"], 4);

    // Defaults are applied in application code, not persisted on read.
    let row = sqlx::query!(
        "SELECT learn_ahead_seconds, day_start_hour FROM user_preferences WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&app.db)
    .await
    .unwrap();
    assert!(row.is_none());
}

/// An update round-trips: PATCH then GET reflects the upserted values (with
/// the wire exposing minutes, converted from the DB's seconds).
#[tokio::test]
async fn preferences_update_round_trips() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(patch(
            &token,
            &serde_json::json!({ "learn_ahead_minutes": 45, "day_start_hour": 6 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["learn_ahead_minutes"], 45);
    assert_eq!(json["day_start_hour"], 6);
}

/// A partial update leaves the other field unchanged.
#[tokio::test]
async fn preferences_partial_update_preserves_other_field() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    // Set both, then update only one.
    let res = app
        .app
        .clone()
        .oneshot(patch(
            &token,
            &serde_json::json!({ "learn_ahead_minutes": 30, "day_start_hour": 5 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(patch(&token, &serde_json::json!({ "day_start_hour": 7 })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // learn_ahead_minutes untouched, day_start_hour updated.
    assert_eq!(json["learn_ahead_minutes"], 30);
    assert_eq!(json["day_start_hour"], 7);
}

/// Out-of-range values are rejected with 400 (learn ahead 0..=100, day hour 0..=23).
#[tokio::test]
async fn preferences_reject_out_of_range() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    for bad in [serde_json::json!({ "learn_ahead_minutes": 101 }), serde_json::json!({ "day_start_hour": 24 })] {
        let res = app.app.clone().oneshot(patch(&token, &bad)).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::BAD_REQUEST,
            "value {bad} should be rejected"
        );
    }
}

/// A request without a valid token is rejected with 401.
#[tokio::test]
async fn preferences_reject_missing_token() {
    let app = common::TestApp::new().await;

    let res = app
        .app
        .clone()
        .oneshot(Request::builder().uri("/preferences").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// US-3.1 — timezone
// ---------------------------------------------------------------------------

/// The default timezone is `"UTC"` before any update, preserving pre-timezone
/// behaviour exactly.
#[tokio::test]
async fn preferences_timezone_defaults_to_utc() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["timezone"], "UTC");
}

/// A valid IANA timezone round-trips through PATCH → GET.
#[tokio::test]
async fn preferences_timezone_round_trips() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(patch(&token, &serde_json::json!({ "timezone": "Europe/London" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["timezone"], "Europe/London");
}

/// An unparseable timezone falls back to `"UTC"` (no 400), and the other
/// fields are still applied.
#[tokio::test]
async fn preferences_timezone_invalid_falls_back_to_utc() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(patch(
            &token,
            &serde_json::json!({ "timezone": "Not/AZone", "day_start_hour": 5 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["timezone"], "UTC");
    assert_eq!(json["day_start_hour"], 5);
}

// ---------------------------------------------------------------------------
// US-3.2 — timebox time limit
// ---------------------------------------------------------------------------

/// The default timebox time limit is `0` (disabled) before any update.
#[tokio::test]
async fn preferences_timebox_defaults_to_zero() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["timebox_time_limit"], 0);
}

/// A timebox time limit round-trips through PATCH → GET (minutes on the wire).
#[tokio::test]
async fn preferences_timebox_round_trips() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    let res = app
        .app
        .clone()
        .oneshot(patch(&token, &serde_json::json!({ "timebox_time_limit": 30 })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preferences")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["timebox_time_limit"], 30);
}

/// Values outside 0..=9999 are rejected with 400.
#[tokio::test]
async fn preferences_timebox_rejects_out_of_range() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_, token) = create_user(&app, UserRole::Student).await;

    for bad in [serde_json::json!({ "timebox_time_limit": -1 }), serde_json::json!({ "timebox_time_limit": 10000 })] {
        let res = app.app.clone().oneshot(patch(&token, &bad)).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::BAD_REQUEST,
            "value {bad} should be rejected"
        );
    }
}
