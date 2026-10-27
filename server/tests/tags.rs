//! Tests for US-6.2/6.3 — note tags (teacher/admin-authored).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::{UserRole, create_user};

fn token_header(token: &str) -> String {
    format!("Bearer {token}")
}

async fn get(app: &common::TestApp, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", token_header(token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn send(
    app: &common::TestApp,
    method: &str,
    path: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let res = app
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", token_header(token))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Seed a teacher-owned deck + note type + note + card, returning the note id.
async fn seed_note(app: &common::TestApp, teacher_id: i64) -> i64 {
    let nt_id = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names) VALUES (1, $1, '[\"Front\",\"Back\"]') RETURNING id",
        uuid::Uuid::new_v4().to_string()
    )
    .fetch_one(&app.db)
    .await
    .expect("insert note type")
    .id;

    let tpl_id = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES ($1, 0, 'Card 1', '{{Front}}', '{{Front}}<hr>{{Back}}') RETURNING id",
        nt_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert template")
    .id;

    let deck_id = sqlx::query!(
        "INSERT INTO decks (school_id, title, created_by) VALUES (1, 'Deck', $1) RETURNING id",
        teacher_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert deck")
    .id;

    let note_id = sqlx::query!(
        "INSERT INTO notes (note_type_id, fields_json) VALUES ($1, '{\"Front\":\"Q\",\"Back\":\"A\"}') RETURNING id",
        nt_id
    )
    .fetch_one(&app.db)
    .await
    .expect("insert note")
    .id;

    sqlx::query!(
        "INSERT INTO cards (note_id, deck_id, template_id) VALUES ($1, $2, $3)",
        note_id,
        deck_id,
        tpl_id
    )
    .execute(&app.db)
    .await
    .expect("insert card");

    note_id
}

/// A teacher can add tags (by name, normalised) and they appear on the note.
#[tokio::test]
async fn add_tags_to_note_and_read_back() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let (status, json) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["Biology", "  Cells::Mitosis ", "Biology"] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // Normalised (trim + lowercase), deduped, sorted by name.
    let names: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["biology", "cells::mitosis"]);

    // The note response now carries the tags.
    let (status2, note_json) = get(&app, &format!("/notes/{note_id}"), &token).await;
    assert_eq!(status2, StatusCode::OK);
    let mut note_tags: Vec<String> = note_json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    note_tags.sort();
    assert_eq!(note_tags, vec!["biology", "cells::mitosis"]);
}

/// Removing tags by name removes only those tags.
#[tokio::test]
async fn remove_tags_from_note() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["a", "b", "c"] }),
    )
    .await;

    let (status, json) = send(
        &app,
        "DELETE",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["B"] }), // normalised to "b"
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["a", "c"]);
}

/// The distinct-tags endpoint lists every tag (the sidebar tree source), and a
/// student is forbidden from mutating tags.
#[tokio::test]
async fn distinct_tags_list_and_student_forbidden() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let (_student_id, student_token) = create_user(&app, UserRole::Student).await;
    let note_id = seed_note(&app, teacher_id).await;

    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["alpha", "alpha::child"] }),
    )
    .await;

    let (status, json) = get(&app, "/tags", &token).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["alpha", "alpha::child"]);

    // A student may not add tags.
    let (s2, _) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &student_token,
        serde_json::json!({ "tags": ["nope"] }),
    )
    .await;
    assert_eq!(s2, StatusCode::FORBIDDEN);
}

/// "Clear unused tags" removes tags no note references.
#[tokio::test]
async fn clear_unused_tags_removes_orphans() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["keep"] }),
    )
    .await;
    // Create an unused tag directly.
    sqlx::query!("INSERT INTO tags (name) VALUES ('orphan')")
        .execute(&app.db)
        .await
        .expect("insert orphan tag");

    let (status, _) = send(&app, "DELETE", "/tags", &token, serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let (_, json) = get(&app, "/tags", &token).await;
    let names: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["keep"], "orphan tag should be cleared");
}

/// Renaming a tag updates the one `tags` row and persists to the note.
#[tokio::test]
async fn rename_tag_updates_name() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let (_, added) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["bio"] }),
    )
    .await;
    let tag_id = added[0]["id"].as_i64().unwrap();

    // Rename "bio" -> "biology::cells".
    let (status, renamed) = send(
        &app,
        "PATCH",
        &format!("/tags/{tag_id}"),
        &token,
        serde_json::json!({ "name": "Biology::Cells" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"].as_str(), Some("biology::cells"), "normalised rename");

    // The note now reports the renamed tag under the same id.
    let (_, note_json) = get(&app, &format!("/notes/{note_id}"), &token).await;
    let names: Vec<String> = note_json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["biology::cells"]);
}

/// Renaming onto an existing tag name *merges* (Anki parity) rather than
/// erroring: the source's note links are folded into the existing target and the
/// source row is removed.
#[tokio::test]
async fn rename_tag_merges_onto_existing_name() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let (_, added) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["a", "b"] }),
    )
    .await;
    let tag_a = added[0]["id"].as_i64().unwrap();
    let tag_b = added[1]["id"].as_i64().unwrap();

    // Rename "a" onto the existing "b": merges "a"'s note link into "b".
    let (status, renamed) = send(
        &app,
        "PATCH",
        &format!("/tags/{tag_a}"),
        &token,
        serde_json::json!({ "name": "b" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "merge onto existing name succeeds");
    assert_eq!(renamed["name"].as_str(), Some("b"));
    assert_eq!(renamed["id"].as_i64(), Some(tag_b), "survives as the existing target id");

    // Only "b" remains; the note now carries a single deduped "b" tag.
    let (_, all) = get(&app, "/tags", &token).await;
    let names: Vec<String> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["b"]);

    let (_, note_json) = get(&app, &format!("/notes/{note_id}"), &token).await;
    let note_tags: Vec<String> = note_json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(note_tags, vec!["b"], "links merged and deduped onto b");
}

/// Renaming a parent renames its whole `::`-subtree (Anki parity).
#[tokio::test]
async fn rename_tag_renames_subtree() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let (_, added) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["bio", "bio::cells", "bio::mitosis::deep", "sibling"] }),
    )
    .await;
    // Find the "bio" id.
    let bio = added
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_str() == Some("bio"))
        .unwrap();
    let bio_id = bio["id"].as_i64().unwrap();

    // Rename "bio" -> "biology": the whole subtree follows; "sibling" is untouched.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/tags/{bio_id}"),
        &token,
        serde_json::json!({ "name": "biology" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, all) = get(&app, "/tags", &token).await;
    let mut names: Vec<String> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "biology",
            "biology::cells",
            "biology::mitosis::deep",
            "sibling"
        ]
    );
}

/// Moving a tag into its own subtree is rejected (the rewrite is undefined).
#[tokio::test]
async fn rename_tag_rejects_move_into_own_subtree() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let (_, added) = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["bio", "bio::cells"] }),
    )
    .await;
    let bio = added
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_str() == Some("bio"))
        .unwrap();
    let bio_id = bio["id"].as_i64().unwrap();

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/tags/{bio_id}"),
        &token,
        serde_json::json!({ "name": "bio::cells" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// `GET /tags/tree` returns the tag namespace as a `::`-split tree with implicit
/// interior nodes.
#[tokio::test]
async fn tag_tree_endpoint_returns_hierarchy() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["bio::cells", "bio::mitosis", "physics"] }),
    )
    .await;

    let (status, json) = get(&app, "/tags/tree", &token).await;
    assert_eq!(status, StatusCode::OK);

    // Roots: bio (interior) and physics (leaf).
    let roots = json.as_array().unwrap();
    let names: Vec<&str> = roots.iter().map(|n| n["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["bio", "physics"]);

    // bio has children cells + mitosis.
    let bio = &roots[0];
    let bio_children: Vec<&str> = bio["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap())
        .collect();
    assert_eq!(bio_children, vec!["cells", "mitosis"]);
}

/// `POST /tags/find-replace` replaces the substring across every tag name,
/// including `::` descendants (Anki parity — US-6.8).
#[tokio::test]
async fn find_replace_tags_rewrites_names() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_id = seed_note(&app, teacher_id).await;

    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_id}/tags"),
        &token,
        serde_json::json!({ "tags": ["bio", "bio::cells", "biology"] }),
    )
    .await;

    // Replace "bio" with "life". This is substring semantics: `bio`, `bio::cells`,
    // and `biology` all contain `bio`.
    let (status, json) = send(
        &app,
        "POST",
        "/tags/find-replace",
        &token,
        serde_json::json!({ "find": "bio", "replace": "life" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {json}");
    assert!(json["message"].as_str().unwrap().contains("3"), "3 tags rewritten: {json}");

    let (_, all) = get(&app, "/tags", &token).await;
    let mut names: Vec<String> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["life", "life::cells", "lifelogy"]);
}

/// Find-and-replace merges on collision: replacing `beta` → `alpha` (where
/// `alpha` already exists) folds `beta`'s note links into `alpha` and drops the
/// vacated `beta` row.
#[tokio::test]
async fn find_replace_tags_merges_on_collision() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let note_a = seed_note(&app, teacher_id).await;
    let note_b = seed_note(&app, teacher_id).await;

    // note_a gets `alpha`, note_b gets `beta`.
    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_a}/tags"),
        &token,
        serde_json::json!({ "tags": ["alpha"] }),
    )
    .await;
    let _ = send(
        &app,
        "POST",
        &format!("/notes/{note_b}/tags"),
        &token,
        serde_json::json!({ "tags": ["beta"] }),
    )
    .await;

    // `beta` -> `alpha` collides with the existing `alpha`: merge + dedup.
    let (status, json) = send(
        &app,
        "POST",
        "/tags/find-replace",
        &token,
        serde_json::json!({ "find": "beta", "replace": "alpha" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {json}");

    // Only the surviving `alpha` remains.
    let (_, all) = get(&app, "/tags", &token).await;
    let names: Vec<String> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["alpha"], "beta merged into alpha");

    // note_b now reports `alpha` (its `beta` link was merged onto `alpha`).
    let (_, note_b_json) = get(&app, &format!("/notes/{note_b}"), &token).await;
    let note_b_tags: Vec<String> = note_b_json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(note_b_tags, vec!["alpha"]);
}

/// An empty find or replace string is rejected.
#[tokio::test]
async fn find_replace_tags_rejects_empty() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (teacher_id, token) = create_user(&app, UserRole::Teacher).await;
    let _note = seed_note(&app, teacher_id).await;

    let (s1, _) = send(
        &app,
        "POST",
        "/tags/find-replace",
        &token,
        serde_json::json!({ "find": "", "replace": "x" }),
    )
    .await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);

    let (s2, _) = send(
        &app,
        "POST",
        "/tags/find-replace",
        &token,
        serde_json::json!({ "find": "x", "replace": "" }),
    )
    .await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);
}
