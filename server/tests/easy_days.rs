//! Tests for US-2.16 — Easy Days (persist-only).

mod common;

use anjuman_contracts::deck_options::EasyDayStrength;
use common::{UserRole, create_user};

/// The global default preset must default all seven days to `normal`.
#[tokio::test]
async fn easy_days_defaults_to_all_normal() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");
    assert_eq!(options.easy_days.len(), 7);
    assert!(options.easy_days.iter().all(|d| *d == EasyDayStrength::Normal));
}

/// A non-default per-weekday selection set at create time survives a read-back.
#[tokio::test]
async fn easy_days_round_trips_through_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("easy days {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({
        "name": name,
        "easy_days": ["normal", "reduced", "minimum", "normal", "normal", "normal", "normal"],
    });
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
    assert_eq!(options.easy_days.len(), 7);
    assert_eq!(options.easy_days[0], EasyDayStrength::Normal);
    assert_eq!(options.easy_days[1], EasyDayStrength::Reduced);
    assert_eq!(options.easy_days[2], EasyDayStrength::Minimum);
    assert_eq!(options.easy_days[3], EasyDayStrength::Normal);
}

/// A PATCH updates the selection and the new values persist.
#[tokio::test]
async fn easy_days_round_trips_through_update() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("easy days {}", uuid::Uuid::new_v4());
    let create = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/deck-options")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "name": name }).to_string()))
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
                    serde_json::json!({
                        "easy_days": ["minimum", "minimum", "normal", "normal", "normal", "normal", "normal"],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch.status(), StatusCode::OK);

    let options = anjuman_server::deck_options::get_options(&app.db, id)
        .await
        .expect("read back");
    assert_eq!(options.easy_days[0], EasyDayStrength::Minimum);
    assert_eq!(options.easy_days[1], EasyDayStrength::Minimum);
    assert_eq!(options.easy_days[2], EasyDayStrength::Normal);
}
