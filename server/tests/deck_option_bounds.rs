//! Tests for deck-option numeric bounds (matching Anki's in-app min/max).

mod common;

use common::{UserRole, create_user};

/// Helper: POST a preset with the given extra JSON field and return the status.
async fn create_with(app: &common::TestApp, token: &str, extra: serde_json::Value) -> axum::http::StatusCode {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let mut body = serde_json::json!({ "name": uuid::Uuid::new_v4().to_string() });
    if let serde_json::Value::Object(map) = &mut body {
        if let serde_json::Value::Object(extra) = extra {
            for (k, v) in extra {
                map.insert(k, v);
            }
        }
    }

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
    res.status()
}

/// Out-of-range values are rejected with 400; in-range values are accepted.
#[tokio::test]
async fn numeric_bounds_are_enforced() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    // (field, bad value, good value)
    let cases: Vec<(&str, serde_json::Value, serde_json::Value)> = vec![
        ("leech_threshold", serde_json::json!(0), serde_json::json!(8)),
        ("leech_threshold", serde_json::json!(10000), serde_json::json!(8)),
        ("new_per_day", serde_json::json!(-1), serde_json::json!(20)),
        ("new_per_day", serde_json::json!(10000), serde_json::json!(20)),
        ("review_per_day", serde_json::json!(-1), serde_json::json!(200)),
        ("review_per_day", serde_json::json!(10000), serde_json::json!(200)),
        ("desired_retention", serde_json::json!(0.5), serde_json::json!(0.9)),
        ("desired_retention", serde_json::json!(1.0), serde_json::json!(0.9)),
        ("auto_advance_seconds_show_question", serde_json::json!(-1.0), serde_json::json!(2.5)),
        ("auto_advance_seconds_show_question", serde_json::json!(10000.0), serde_json::json!(2.5)),
        ("auto_advance_seconds_show_answer", serde_json::json!(-1.0), serde_json::json!(2.5)),
        ("auto_advance_seconds_show_answer", serde_json::json!(10000.0), serde_json::json!(2.5)),
    ];

    for (field, bad, good) in cases {
        let bad_status =
            create_with(&app, &token, serde_json::json!({ field: bad })).await;
        assert_eq!(bad_status, axum::http::StatusCode::BAD_REQUEST, "{field}={bad} should be rejected");
        let good_status =
            create_with(&app, &token, serde_json::json!({ field: good })).await;
        assert_eq!(good_status, axum::http::StatusCode::CREATED, "{field}={good} should be accepted");
    }
}
