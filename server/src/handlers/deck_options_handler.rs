// Deck options preset management.
//
// Teachers and admins can create, view, update, and delete deck options
// presets. Presets are scoped to a school, and decks reference a preset via
// `options_id`.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use anjuman_contracts::deck_options::{CreateDeckOptions, UpdateDeckOptions};
use anjuman_contracts::UserRole;

use crate::{
    auth::AuthUser,
    db_types::DbLeechAction,
    deck_options::{self, DeckOptions},
    handlers::{reviews::parse_steps},
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
            "Only teachers and admins can manage deck options",
        )),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /deck-options` — List all presets in the school.
pub async fn list_deck_options(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DeckOptions>>, (StatusCode, &'static str)> {
    let rows = sqlx::query!(
        "SELECT id FROM deck_options WHERE school_id = $1 ORDER BY name",
        claims.school_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let mut options = Vec::new();
    for row in rows {
        let opt = deck_options::get_options(&state.db, row.id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        options.push(opt);
    }

    Ok(Json(options))
}

/// `GET /deck-options/:id` — Get a single preset.
pub async fn get_deck_options(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeckOptions>, (StatusCode, &'static str)> {
    let opt = deck_options::get_options(&state.db, id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Deck options not found"))?;

    if opt.school_id != claims.school_id {
        return Err((StatusCode::NOT_FOUND, "Deck options not found"));
    }

    Ok(Json(opt))
}

/// `POST /deck-options` — Create a new preset.
pub async fn create_deck_options(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateDeckOptions>,
) -> Result<(StatusCode, Json<DeckOptions>), (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;

    if body.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }

    let learning_steps =
        parse_steps(&body.learning_steps).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let relearning_steps =
        parse_steps(&body.relearning_steps).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Learning steps are mandatory (empty would make new cards unlearnable);
    // empty relearning steps are allowed (FSRS: skip relearning, US-2.4).
    if learning_steps.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Learning steps cannot be empty".to_string(),
        ));
    }

    let bury_new = body.bury_new;
    let bury_review = body.bury_review;
    let bury_interday = body.bury_interday;

    // Create the preset and its steps atomically.
    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let result = sqlx::query!(
        "INSERT INTO deck_options (school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, leech_threshold, leech_action) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::text::leech_action) RETURNING id",
        claims.school_id,
        body.name,
        body.desired_retention,
        bury_new,
        bury_review,
        bury_interday,
        body.new_per_day,
        body.review_per_day,
        body.leech_threshold,
        DbLeechAction::from(body.leech_action).as_str(),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            (
                StatusCode::CONFLICT,
                "A deck options preset with that name already exists".to_string(),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        }
    })?;

    let new_id = result.id;
    deck_options::replace_steps(&mut tx, new_id, &learning_steps, &relearning_steps)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to persist steps".to_string(),
            )
        })?;

    tx.commit().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    let opt = deck_options::get_options(&state.db, new_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch created deck options".to_string(),
            )
        })?;

    Ok((StatusCode::CREATED, Json(opt)))
}

/// `PATCH /deck-options/:id` — Update a preset.
pub async fn update_deck_options(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateDeckOptions>,
) -> Result<Json<DeckOptions>, (StatusCode, String)> {
    check_teacher_or_admin(&claims).map_err(|(s, m)| (s, m.to_string()))?;

    let existing = deck_options::get_options(&state.db, id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Deck options not found".to_string()))?;
    if existing.school_id != claims.school_id {
        return Err((StatusCode::NOT_FOUND, "Deck options not found".to_string()));
    }

    // Validate inputs up front (before opening the transaction).
    if let Some(name) = &body.name
        && name.is_empty()
    {
        return Err((StatusCode::BAD_REQUEST, "Name cannot be empty".to_string()));
    }
    if let Some(retention) = body.desired_retention
        && !(0.0..=1.0).contains(&retention)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "desired_retention must be between 0 and 1".to_string(),
        ));
    }

    // Resolve final step lists if either is being updated.
    let mut final_learning = existing.learning_steps.clone();
    let mut final_relearning = existing.relearning_steps.clone();
    let mut steps_changed = false;
    if let Some(steps) = &body.learning_steps {
        final_learning = parse_steps(steps).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        if final_learning.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Learning steps cannot be empty".to_string(),
            ));
        }
        steps_changed = true;
    }
    if let Some(steps) = &body.relearning_steps {
        final_relearning = parse_steps(steps).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        steps_changed = true;
    }

    // Apply every field change (and, if steps changed, the delete+insert of
    // steps) in a single atomic transaction.
    let mut tx = crate::db::begin_immediate(&state.db).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    })?;

    if let Some(name) = &body.name {
        sqlx::query!("UPDATE deck_options SET name = $1 WHERE id = $2", name, id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    (
                        StatusCode::CONFLICT,
                        "A deck options preset with that name already exists".to_string(),
                    )
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error".to_string(),
                    )
                }
            })?;
    }

    if let Some(retention) = body.desired_retention {
        sqlx::query!(
            "UPDATE deck_options SET desired_retention = $1 WHERE id = $2",
            retention,
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

    if let Some(v) = body.bury_new {
        sqlx::query!(
            "UPDATE deck_options SET bury_new = $1 WHERE id = $2",
            v,
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

    if let Some(v) = body.bury_review {
        sqlx::query!(
            "UPDATE deck_options SET bury_review = $1 WHERE id = $2",
            v,
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

    if let Some(v) = body.bury_interday {
        sqlx::query!(
            "UPDATE deck_options SET bury_interday = $1 WHERE id = $2",
            v,
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

    if let Some(v) = body.new_per_day {
        sqlx::query!(
            "UPDATE deck_options SET new_per_day = $1 WHERE id = $2",
            v,
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

    if let Some(v) = body.review_per_day {
        sqlx::query!(
            "UPDATE deck_options SET review_per_day = $1 WHERE id = $2",
            v,
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

    if let Some(v) = body.leech_threshold {
        sqlx::query!(
            "UPDATE deck_options SET leech_threshold = $1 WHERE id = $2",
            v,
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

    if let Some(action) = body.leech_action {
        sqlx::query!(
            "UPDATE deck_options SET leech_action = $1::text::leech_action WHERE id = $2",
            DbLeechAction::from(action).as_str(),
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

    if steps_changed {
        deck_options::replace_steps(&mut tx, id, &final_learning, &final_relearning)
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

    let opt = deck_options::get_options(&state.db, id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch updated deck options".to_string(),
            )
        })?;

    Ok(Json(opt))
}

/// `DELETE /deck-options/:id` — Delete a preset.
///
/// Decks referencing this preset fall back to defaults (options_id = NULL).
pub async fn delete_deck_options(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;

    let existing = deck_options::get_options(&state.db, id)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Deck options not found"))?;
    if existing.school_id != claims.school_id {
        return Err((StatusCode::NOT_FOUND, "Deck options not found"));
    }

    sqlx::query!("DELETE FROM deck_options WHERE id = $1", id)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(
        serde_json::json!({ "message": "Deck options deleted" }),
    ))
}
