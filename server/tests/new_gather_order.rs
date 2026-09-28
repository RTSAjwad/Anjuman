//! Tests for US-2.8 — new card gather order (enum plumbing + defaults).

mod common;

use anjuman_server::deck_options::get_options;

use common::{UserRole, create_user};

#[tokio::test]
async fn new_gather_order_defaults_to_deck() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_student_id, _token) = create_user(&app, UserRole::Teacher).await;

    // The global default preset (id 0) must default to `deck`.
    let options = get_options(&app.db, 0).await.expect("default preset");
    assert_eq!(options.new_gather_order, anjuman_contracts::deck_options::NewGatherOrder::Deck);
}

#[tokio::test]
async fn new_gather_order_round_trips_through_the_handler() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_student_id, token) = create_user(&app, UserRole::Teacher).await;

    // Create a preset with a non-default gather order, then read it back.
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("Random Cards {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({ "name": name, "new_gather_order": "random_cards" });
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
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["new_gather_order"], "random_cards");
    let id = json["id"].as_i64().unwrap();

    // And the resolved options read `random_cards` back.
    let options = get_options(&app.db, id).await.expect("read back");
    assert_eq!(
        options.new_gather_order,
        anjuman_contracts::deck_options::NewGatherOrder::RandomCards
    );
}

#[tokio::test]
async fn deck_then_random_notes_is_accepted() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_student_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let body = serde_json::json!({ "name": uuid::Uuid::new_v4().to_string(), "new_gather_order": "deck_then_random_notes" });
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
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["new_gather_order"], "deck_then_random_notes");
}
