// Note-tag management (US-6.2/6.3).
//
//   GET    /tags                  — the distinct tag namespace (sidebar tree source)
//   POST   /notes/{id}/tags       — add tags to a note (by name, normalised)
//   DELETE /notes/{id}/tags       — remove tags from a note (by name)
//   DELETE /tags                  — clear unused tags
//
// Tags are teacher/admin-authored (shared note taxonomy); students read a note's
// tags via `NoteResponse.tags` but never mutate them.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use anjuman_contracts::tags::{AddTags, RemoveTags, RenameTag, Tag, TagNode, UnusedTagsResponse};
use anjuman_contracts::MessageResponse;

use crate::{
    auth::AuthUser,
    handlers::notes,
    state::AppState,
    tags,
};

fn check_teacher_or_admin(claims: &crate::auth::Claims) -> Result<(), (StatusCode, &'static str)> {
    crate::permissions::require_teacher_or_admin(claims, "Only teachers and admins can manage tags")
}

/// `GET /tags` — the distinct tag namespace (every tag, by name).
pub async fn list_tags(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<Tag>>, (StatusCode, &'static str)> {
    let _ = &claims; // any authenticated user may list tags (visibility follows notes)
    let tags = tags::all_tags(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    Ok(Json(tags))
}

/// `GET /tags/tree` — the distinct tag namespace as a `::`-split tree (the
/// sidebar shape, with implicit interior nodes).
pub async fn tag_tree(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<TagNode>>, (StatusCode, &'static str)> {
    let _ = &claims;
    let all = tags::all_tags(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    let names: Vec<String> = all.into_iter().map(|t| t.name).collect();
    let tree = tags::build_tag_tree(names.iter().map(|s| s.as_str()));
    Ok(Json(tree))
}

/// `GET /tags/stats` — the number of unused (orphan) tags, for gating the
/// "Clear Unused Tags" affordance.
pub async fn unused_tags_stats(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UnusedTagsResponse>, (StatusCode, &'static str)> {
    let _ = &claims;
    let unused = tags::count_unused_tags(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    Ok(Json(UnusedTagsResponse { unused }))
}

/// `POST /notes/{id}/tags` — add tags (by name) to a note.
pub async fn add_tags(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
    Json(body): Json<AddTags>,
) -> Result<Json<Vec<Tag>>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    notes::check_note_authorization(&state.db, note_id, claims.school_id, &claims)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    // Normalise + dedup the requested tag names (preserving first-seen order),
    // rejecting empty names.
    let mut names: Vec<String> = Vec::new();
    for raw in &body.tags {
        let n = tags::normalize_name(raw);
        if n.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Tag name cannot be empty".to_string()));
        }
        if !names.contains(&n) {
            names.push(n);
        }
    }

    let mut tx = crate::db::begin_immediate(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    for name in &names {
        let tag_id = tags::ensure_tag(&mut tx, name)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
        sqlx::query!(
            "INSERT INTO note_tags (note_id, tag_id) VALUES ($1, $2) ON CONFLICT (note_id, tag_id) DO NOTHING",
            note_id,
            tag_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    let all = tags::tags_for_note(&state.db, note_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    Ok(Json(all))
}

/// `DELETE /notes/{id}/tags` — remove tags (by name) from a note.
pub async fn remove_tags(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
    Json(body): Json<RemoveTags>,
) -> Result<Json<Vec<Tag>>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    notes::check_note_authorization(&state.db, note_id, claims.school_id, &claims)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    let mut tx = crate::db::begin_immediate(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    for raw in &body.tags {
        let n = tags::normalize_name(raw);
        if n.is_empty() {
            continue;
        }
        sqlx::query!(
            "DELETE FROM note_tags WHERE note_id = $1 AND tag_id = (SELECT id FROM tags WHERE name = $2)",
            note_id,
            n
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    let remaining = tags::tags_for_note(&state.db, note_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    Ok(Json(remaining))
}

/// `DELETE /tags` — clear unused tags (remove `tags` rows with no note link).
pub async fn clear_unused_tags(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    let removed = tags::clear_unused_tags(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    Ok(Json(MessageResponse {
        message: format!("Removed {removed} unused tag(s)"),
    }))
}

/// `PATCH /tags/{id}` — rename a tag and its whole `::`-subtree, merging onto an
/// existing target name (Anki parity). The `{id}` picks the tag to rename; its
/// current name is resolved here and the subtree rename operates by name.
pub async fn rename_tag(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(tag_id): Path<i64>,
    Json(body): Json<RenameTag>,
) -> Result<Json<Tag>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;

    let normalized = tags::normalize_name(&body.name);
    if normalized.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Tag name cannot be empty".to_string()));
    }

    // Resolve the tag to rename by id → current name.
    let old = sqlx::query!("SELECT name FROM tags WHERE id = $1", tag_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Tag not found".to_string()))?;

    // Reject a rename where the target is the tag itself or one of its own
    // descendants (`foo` → `foo::bar`): the subtree rewrite is undefined there.
    if normalized == old.name || normalized.starts_with(&format!("{}::", old.name)) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot move a tag into its own subtree".to_string(),
        ));
    }

    let renamed = tags::rename_tag_subtree(&state.db, &old.name, &normalized)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Tag not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
        })?;

    Ok(Json(renamed))
}
