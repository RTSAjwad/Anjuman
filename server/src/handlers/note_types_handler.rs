// Note type and card template management.
//
// Note types are not hand-crafted from scratch: new note types are created by
// cloning an existing note type (which copies its templates). Built-in note
// types are seeded. Templates are managed independently via their own CRUD
// endpoints, keyed by a stable template id.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use anjuman_contracts::note_types::{
    CreateTemplate, NoteTypeResponse, ReorderTemplates, UpdateNoteType, UpdateTemplate,
};
use anjuman_contracts::{MessageResponse, UserRole};

use crate::{
    auth::AuthUser,
    note_types::{self, Template},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_teacher_or_admin(claims: &crate::auth::Claims) -> Result<(), (StatusCode, &'static str)> {
    match claims.role {
        UserRole::Admin | UserRole::Teacher => Ok(()),
        UserRole::Student => Err((
            StatusCode::FORBIDDEN,
            "Only teachers and admins can manage note types",
        )),
    }
}

/// Verify a note type exists in the caller's school and return its row id.
async fn check_note_type_owner(
    db: &sqlx::SqlitePool,
    note_type_id: i64,
    school_id: i64,
) -> Result<(), (StatusCode, &'static str)> {
    let exists = sqlx::query!(
        "SELECT id FROM note_types WHERE id = ? AND school_id = ?",
        note_type_id,
        school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "Note type not found"));
    }
    Ok(())
}

/// Build a `NoteTypeResponse` from a `NoteType`, resolving the note count.
async fn to_response(
    db: &sqlx::SqlitePool,
    nt: note_types::NoteType,
) -> Result<NoteTypeResponse, (StatusCode, &'static str)> {
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM notes WHERE note_type_id = ?"#,
        nt.id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(NoteTypeResponse {
        id: nt.id,
        name: nt.name,
        field_names: nt.field_names,
        sort_field: nt.sort_field,
        templates: nt.templates,
        note_count: count,
    })
}

/// The next `ord` for a template appended to a note type.
async fn next_ord(
    db: &sqlx::SqlitePool,
    note_type_id: i64,
) -> Result<i64, (StatusCode, &'static str)> {
    let max_ord: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(ord), -1) FROM note_type_templates WHERE note_type_id = ?",
        note_type_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(max_ord + 1)
}

// ---------------------------------------------------------------------------
// Note type handlers
// ---------------------------------------------------------------------------

/// `GET /note-types` — List all note types in the school.
pub async fn list_note_types(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<NoteTypeResponse>>, (StatusCode, &'static str)> {
    let types = note_types::list_note_types(&state.db, claims.school_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let mut responses = Vec::new();
    for nt in types {
        responses.push(to_response(&state.db, nt).await?);
    }

    Ok(Json(responses))
}

/// `GET /note-types/{id}` — Get a single note type.
pub async fn get_note_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NoteTypeResponse>, (StatusCode, &'static str)> {
    check_note_type_owner(&state.db, id, claims.school_id).await?;

    let nt = note_types::get_note_type(&state.db, id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Note type not found"))?;

    Ok(Json(to_response(&state.db, nt).await?))
}

/// `POST /note-types/{id}/clone` — Clone an existing note type (and its
/// templates) into a new, empty note type.
pub async fn clone_note_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<NoteTypeResponse>), (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;

    // Verify the source note type exists in the caller's school.
    check_note_type_owner(&state.db, id, claims.school_id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    let source = note_types::get_note_type(&state.db, id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Note type not found".to_string()))?;

    let new_name = format!("{} (copy)", source.name);

    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let source_field_names_json = serde_json::to_string(&source.field_names).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid field names".to_string(),
        )
    })?;

    let result = sqlx::query!(
        "INSERT INTO note_types (school_id, name, field_names, sort_field, created_by, created_at) VALUES (?, ?, ?, ?, ?, unixepoch())",
        claims.school_id,
        new_name,
        source_field_names_json,
        source.sort_field,
        claims.sub,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            (StatusCode::CONFLICT, "A note type with that name already exists".to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
        }
    })?;

    let new_id = result.last_insert_rowid();

    // Copy templates (new ids), preserving order.
    for template in &source.templates {
        let ord = template.ord;
        sqlx::query!(
            "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES (?, ?, ?, ?, ?)",
            new_id,
            ord,
            template.name,
            template.front_pattern,
            template.back_pattern,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let nt = note_types::get_note_type(&state.db, new_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch cloned note type".to_string(),
            )
        })?;

    let resp = to_response(&state.db, nt)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    Ok((StatusCode::CREATED, Json(resp)))
}

/// `PATCH /note-types/{id}` — Update a note type's name, fields, or sort field
/// (not its templates).
pub async fn update_note_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateNoteType>,
) -> Result<Json<NoteTypeResponse>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    check_note_type_owner(&state.db, id, claims.school_id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    if let Some(name) = &body.name
        && name.is_empty()
    {
        return Err((StatusCode::BAD_REQUEST, "Name cannot be empty".to_string()));
    }
    if let Some(field_names) = &body.field_names
        && field_names.is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one field is required".to_string(),
        ));
    }

    let field_names_json = match &body.field_names {
        Some(field_names) => Some(
            serde_json::to_string(field_names)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid field names".to_string()))?,
        ),
        None => None,
    };

    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    if let Some(name) = &body.name {
        sqlx::query!("UPDATE note_types SET name = ? WHERE id = ?", name, id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    (
                        StatusCode::CONFLICT,
                        "A note type with that name already exists".to_string(),
                    )
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error".to_string(),
                    )
                }
            })?;
    }
    if let Some(json) = &field_names_json {
        sqlx::query!(
            "UPDATE note_types SET field_names = ? WHERE id = ?",
            json,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }
    if let Some(sort_field) = &body.sort_field {
        sqlx::query!(
            "UPDATE note_types SET sort_field = ? WHERE id = ?",
            sort_field,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let nt = note_types::get_note_type(&state.db, id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let resp = to_response(&state.db, nt)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;
    Ok(Json(resp))
}

/// `DELETE /note-types/{id}` — Delete a note type and all its templates,
/// notes, and cards (via cascades).
pub async fn delete_note_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;

    let result = sqlx::query!(
        "DELETE FROM note_types WHERE id = ? AND school_id = ?",
        id,
        claims.school_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Note type not found"));
    }

    Ok(Json(MessageResponse {
        message: "Note type deleted".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Template handlers (independent CRUD)
// ---------------------------------------------------------------------------

/// `POST /note-types/{id}/templates` — Append a template (generate cards for
/// existing notes).
pub async fn create_template(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<CreateTemplate>,
) -> Result<(StatusCode, Json<Template>), (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    check_note_type_owner(&state.db, id, claims.school_id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    if body.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }

    let ord = next_ord(&state.db, id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let result = sqlx::query!(
        "INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern) VALUES (?, ?, ?, ?, ?)",
        id,
        ord,
        body.name,
        body.front_pattern,
        body.back_pattern,
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    let template_id = result.last_insert_rowid();

    // Generate a card for every existing note, in the note's first card's deck.
    sqlx::query!(
        r#"
        INSERT INTO cards (note_id, deck_id, template_id, created_at)
        SELECT n.id,
               (SELECT c.deck_id FROM cards c WHERE c.note_id = n.id ORDER BY c.id ASC LIMIT 1),
               ?,
               unixepoch()
        FROM notes n
        WHERE n.note_type_id = ?
        "#,
        template_id,
        id,
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(Template {
            id: template_id,
            ord,
            name: body.name,
            front_pattern: body.front_pattern,
            back_pattern: body.back_pattern,
        }),
    ))
}

/// `PATCH /note-types/{id}/templates/{template_id}` — Edit a template's
/// patterns/name. No cards are added or removed (they re-render).
pub async fn update_template(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((id, template_id)): Path<(i64, i64)>,
    Json(body): Json<UpdateTemplate>,
) -> Result<Json<Template>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    check_note_type_owner(&state.db, id, claims.school_id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    if let Some(name) = &body.name {
        sqlx::query!(
            "UPDATE note_type_templates SET name = ? WHERE id = ? AND note_type_id = ?",
            name,
            template_id,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }
    if let Some(fp) = &body.front_pattern {
        sqlx::query!(
            "UPDATE note_type_templates SET front_pattern = ? WHERE id = ? AND note_type_id = ?",
            fp,
            template_id,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }
    if let Some(bp) = &body.back_pattern {
        sqlx::query!(
            "UPDATE note_type_templates SET back_pattern = ? WHERE id = ? AND note_type_id = ?",
            bp,
            template_id,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let row = sqlx::query!(
        "SELECT id, ord, name, front_pattern, back_pattern FROM note_type_templates WHERE id = ? AND note_type_id = ?",
        template_id,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    Ok(Json(Template {
        id: row.id,
        ord: row.ord,
        name: row.name,
        front_pattern: row.front_pattern,
        back_pattern: row.back_pattern,
    }))
}

/// `DELETE /note-types/{id}/templates/{template_id}` — Delete a template and
/// its cards.
pub async fn delete_template(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((id, template_id)): Path<(i64, i64)>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;

    let result = sqlx::query!(
        "DELETE FROM note_type_templates WHERE id = ? AND note_type_id = ?",
        template_id,
        id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Template not found"));
    }

    // Cards for this template are deleted via ON DELETE CASCADE.

    Ok(Json(MessageResponse {
        message: "Template deleted".to_string(),
    }))
}

/// `PATCH /note-types/{id}/templates/order` — Reorder templates by `ord`.
pub async fn reorder_templates(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReorderTemplates>,
) -> Result<Json<Vec<Template>>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;
    check_note_type_owner(&state.db, id, claims.school_id)
        .await
        .map_err(|(s, m)| (s, m.to_string()))?;

    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    // Two-phase reorder to avoid violating UNIQUE(note_type_id, ord) while
    // moving ord values around. First bump every ord by a large offset, then
    // apply the final contiguous order.
    const OFFSET: i64 = 1_000_000;
    sqlx::query!(
        "UPDATE note_type_templates SET ord = ord + ? WHERE note_type_id = ?",
        OFFSET,
        id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    for (ord, template_id) in body.template_ids.iter().enumerate() {
        let ord = ord as i64;
        sqlx::query!(
            "UPDATE note_type_templates SET ord = ? WHERE id = ? AND note_type_id = ?",
            ord,
            template_id,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    }

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let nt = note_types::get_note_type(&state.db, id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    Ok(Json(nt.templates))
}
