//! Tests for US-2.14 — selection-only (client-side) deck options.
//!
//! These options have no server-side behaviour; the tests assert persistence
//! (round-trip through create → read and update → read), not any effect.

mod common;

use common::{UserRole, create_user};

/// The global default preset must expose the in-app defaults for every
/// selection-only option.
#[tokio::test]
async fn selection_only_options_defaults() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, _token) = create_user(&app, UserRole::Teacher).await;

    let options = anjuman_server::deck_options::get_options(&app.db, 0)
        .await
        .expect("default preset");

    assert!(!options.show_on_screen_timer);
    assert!(!options.stop_timer_on_answer);
    assert!(!options.dont_play_audio_automatically);
    assert!(!options.skip_question_when_replaying_answer);
    assert_eq!(options.auto_advance_seconds_show_question, 0.0);
    assert_eq!(options.auto_advance_seconds_show_answer, 0.0);
    assert!(options.auto_advance_wait_for_audio);
    assert_eq!(
        options.auto_advance_question_action,
        anjuman_contracts::deck_options::AutoAdvanceQuestionAction::ShowAnswer
    );
    assert_eq!(
        options.auto_advance_answer_action,
        anjuman_contracts::deck_options::AutoAdvanceAnswerAction::BuryCard
    );
}

/// Non-default selection-only values set at create time survive a read-back.
#[tokio::test]
async fn selection_only_options_round_trip_on_create() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("selection {}", uuid::Uuid::new_v4());
    let body = serde_json::json!({
        "name": name,
        "show_on_screen_timer": true,
        "stop_timer_on_answer": true,
        "dont_play_audio_automatically": true,
        "skip_question_when_replaying_answer": true,
        "auto_advance_seconds_show_question": 3.5,
        "auto_advance_seconds_show_answer": 2.0,
        "auto_advance_wait_for_audio": false,
        "auto_advance_question_action": "show_card",
        "auto_advance_answer_action": "answer_hard",
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

    // Read back from the DB and assert every field.
    let options = anjuman_server::deck_options::get_options(&app.db, id)
        .await
        .expect("read back");

    assert!(options.show_on_screen_timer);
    assert!(options.stop_timer_on_answer);
    assert!(options.dont_play_audio_automatically);
    assert!(options.skip_question_when_replaying_answer);
    assert_eq!(options.auto_advance_seconds_show_question, 3.5);
    assert_eq!(options.auto_advance_seconds_show_answer, 2.0);
    assert!(!options.auto_advance_wait_for_audio);
    assert_eq!(
        options.auto_advance_question_action,
        anjuman_contracts::deck_options::AutoAdvanceQuestionAction::ShowCard
    );
    assert_eq!(
        options.auto_advance_answer_action,
        anjuman_contracts::deck_options::AutoAdvanceAnswerAction::AnswerHard
    );
}

/// A PATCH updates the selection-only fields and the new values persist.
#[tokio::test]
async fn selection_only_options_round_trip_on_update() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (_teacher_id, token) = create_user(&app, UserRole::Teacher).await;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let name = format!("selection {}", uuid::Uuid::new_v4());
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
                        "show_on_screen_timer": true,
                        "auto_advance_seconds_show_answer": 7.25,
                        "auto_advance_answer_action": "show_reminder",
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

    assert!(options.show_on_screen_timer);
    assert_eq!(options.auto_advance_seconds_show_answer, 7.25);
    assert_eq!(
        options.auto_advance_answer_action,
        anjuman_contracts::deck_options::AutoAdvanceAnswerAction::ShowReminder
    );
    // Untouched fields keep their defaults.
    assert!(!options.stop_timer_on_answer);
    assert!(options.auto_advance_wait_for_audio);
}
