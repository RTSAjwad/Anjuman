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
    db_types::{DbAutoAdvanceAnswerAction, DbAutoAdvanceQuestionAction, DbInsertionOrder,
        DbInterdayOrder, DbLeechAction, DbNewGatherOrder, DbNewReviewOrder, DbNewSortOrder,
        DbReviewSortOrder},
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

    // The global default preset (id 0) is owned by the system school and shared
    // across every school (a deck with no explicit `options_id` resolves to it —
    // see US-4.9). It is therefore exempt from the per-school scoping check;
    // every other preset is only visible within its own school.
    if id != 0 && opt.school_id != claims.school_id {
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

    if !(1..=7200).contains(&body.maximum_answer_seconds) {
        return Err((
            StatusCode::BAD_REQUEST,
            "maximum_answer_seconds must be between 1 and 7200".to_string(),
        ));
    }
    if !(1..=36500).contains(&body.maximum_interval) {
        return Err((
            StatusCode::BAD_REQUEST,
            "maximum_interval must be between 1 and 36500".to_string(),
        ));
    }

    // Numeric bounds (matching Anki's in-app min/max).
    if !(1..=9999).contains(&body.leech_threshold) {
        return Err((
            StatusCode::BAD_REQUEST,
            "leech_threshold must be between 1 and 9999".to_string(),
        ));
    }
    if !(0..=9999).contains(&body.new_per_day) {
        return Err((
            StatusCode::BAD_REQUEST,
            "new_per_day must be between 0 and 9999".to_string(),
        ));
    }
    if !(0..=9999).contains(&body.review_per_day) {
        return Err((
            StatusCode::BAD_REQUEST,
            "review_per_day must be between 0 and 9999".to_string(),
        ));
    }
    if !(0.70..=0.99).contains(&body.desired_retention) {
        return Err((
            StatusCode::BAD_REQUEST,
            "desired_retention must be between 0.70 and 0.99".to_string(),
        ));
    }
    if !(0.0..=9999.0).contains(&body.auto_advance_seconds_show_question) {
        return Err((
            StatusCode::BAD_REQUEST,
            "auto_advance_seconds_show_question must be between 0 and 9999".to_string(),
        ));
    }
    if !(0.0..=9999.0).contains(&body.auto_advance_seconds_show_answer) {
        return Err((
            StatusCode::BAD_REQUEST,
            "auto_advance_seconds_show_answer must be between 0 and 9999".to_string(),
        ));
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
        "INSERT INTO deck_options (school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, leech_threshold, leech_action, new_gather_order, new_sort_order, new_review_order, interday_order, review_sort_order, show_on_screen_timer, stop_timer_on_answer, dont_play_audio_automatically, skip_question_when_replaying_answer, auto_advance_seconds_show_question, auto_advance_seconds_show_answer, auto_advance_wait_for_audio, auto_advance_question_action, auto_advance_answer_action, maximum_answer_seconds, maximum_interval, easy_days, insertion_order, fsrs_parameters) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::text::leech_action, $11::text::new_gather_order, $12::text::new_sort_order, $13::text::new_review_order, $14::text::interday_order, $15::text::review_sort_order, $16, $17, $18, $19, $20, $21, $22, $23::text::auto_advance_question_action, $24::text::auto_advance_answer_action, $25, $26, $27, $28::text::insertion_order, $29) RETURNING id",
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
        DbNewGatherOrder::from(body.new_gather_order).as_str(),
        DbNewSortOrder::from(body.new_sort_order).as_str(),
        DbNewReviewOrder::from(body.new_review_order).as_str(),
        DbInterdayOrder::from(body.interday_order).as_str(),
        DbReviewSortOrder::from(body.review_sort_order).as_str(),
        body.show_on_screen_timer,
        body.stop_timer_on_answer,
        body.dont_play_audio_automatically,
        body.skip_question_when_replaying_answer,
        body.auto_advance_seconds_show_question,
        body.auto_advance_seconds_show_answer,
        body.auto_advance_wait_for_audio,
        DbAutoAdvanceQuestionAction::from(body.auto_advance_question_action).as_str(),
        DbAutoAdvanceAnswerAction::from(body.auto_advance_answer_action).as_str(),
        body.maximum_answer_seconds,
        body.maximum_interval,
        serde_json::to_value(&body.easy_days).unwrap_or(serde_json::Value::Null),
        DbInsertionOrder::from(body.insertion_order).as_str(),
        serde_json::to_value(&body.fsrs_parameters).unwrap_or(serde_json::Value::Null),
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
        && !(0.70..=0.99).contains(&retention)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "desired_retention must be between 0.70 and 0.99".to_string(),
        ));
    }
    if let Some(cap) = body.maximum_answer_seconds
        && !(1..=7200).contains(&cap)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "maximum_answer_seconds must be between 1 and 7200".to_string(),
        ));
    }
    if let Some(interval) = body.maximum_interval
        && !(1..=36500).contains(&interval)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "maximum_interval must be between 1 and 36500".to_string(),
        ));
    }
    if let Some(v) = body.leech_threshold
        && !(1..=9999).contains(&v)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "leech_threshold must be between 1 and 9999".to_string(),
        ));
    }
    if let Some(v) = body.new_per_day
        && !(0..=9999).contains(&v)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "new_per_day must be between 0 and 9999".to_string(),
        ));
    }
    if let Some(v) = body.review_per_day
        && !(0..=9999).contains(&v)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "review_per_day must be between 0 and 9999".to_string(),
        ));
    }
    if let Some(v) = body.auto_advance_seconds_show_question
        && !(0.0..=9999.0).contains(&v)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "auto_advance_seconds_show_question must be between 0 and 9999".to_string(),
        ));
    }
    if let Some(v) = body.auto_advance_seconds_show_answer
        && !(0.0..=9999.0).contains(&v)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "auto_advance_seconds_show_answer must be between 0 and 9999".to_string(),
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

    if let Some(order) = body.new_gather_order {
        sqlx::query!(
            "UPDATE deck_options SET new_gather_order = $1::text::new_gather_order WHERE id = $2",
            DbNewGatherOrder::from(order).as_str(),
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

    if let Some(order) = body.new_sort_order {
        sqlx::query!(
            "UPDATE deck_options SET new_sort_order = $1::text::new_sort_order WHERE id = $2",
            DbNewSortOrder::from(order).as_str(),
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

    if let Some(order) = body.new_review_order {
        sqlx::query!(
            "UPDATE deck_options SET new_review_order = $1::text::new_review_order WHERE id = $2",
            DbNewReviewOrder::from(order).as_str(),
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

    if let Some(order) = body.interday_order {
        sqlx::query!(
            "UPDATE deck_options SET interday_order = $1::text::interday_order WHERE id = $2",
            DbInterdayOrder::from(order).as_str(),
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

    if let Some(order) = body.review_sort_order {
        sqlx::query!(
            "UPDATE deck_options SET review_sort_order = $1::text::review_sort_order WHERE id = $2",
            DbReviewSortOrder::from(order).as_str(),
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

    // Selection-only (client-side) options — US-2.14. Boolean and numeric
    // fields first, then the two enum actions.
    if let Some(v) = body.show_on_screen_timer {
        sqlx::query!(
            "UPDATE deck_options SET show_on_screen_timer = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.stop_timer_on_answer {
        sqlx::query!(
            "UPDATE deck_options SET stop_timer_on_answer = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.dont_play_audio_automatically {
        sqlx::query!(
            "UPDATE deck_options SET dont_play_audio_automatically = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.skip_question_when_replaying_answer {
        sqlx::query!(
            "UPDATE deck_options SET skip_question_when_replaying_answer = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.auto_advance_seconds_show_question {
        sqlx::query!(
            "UPDATE deck_options SET auto_advance_seconds_show_question = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.auto_advance_seconds_show_answer {
        sqlx::query!(
            "UPDATE deck_options SET auto_advance_seconds_show_answer = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(v) = body.auto_advance_wait_for_audio {
        sqlx::query!(
            "UPDATE deck_options SET auto_advance_wait_for_audio = $1 WHERE id = $2",
            v,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(action) = body.auto_advance_question_action {
        sqlx::query!(
            "UPDATE deck_options SET auto_advance_question_action = $1::text::auto_advance_question_action WHERE id = $2",
            DbAutoAdvanceQuestionAction::from(action).as_str(),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(action) = body.auto_advance_answer_action {
        sqlx::query!(
            "UPDATE deck_options SET auto_advance_answer_action = $1::text::auto_advance_answer_action WHERE id = $2",
            DbAutoAdvanceAnswerAction::from(action).as_str(),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(cap) = body.maximum_answer_seconds {
        sqlx::query!(
            "UPDATE deck_options SET maximum_answer_seconds = $1 WHERE id = $2",
            cap,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(interval) = body.maximum_interval {
        sqlx::query!(
            "UPDATE deck_options SET maximum_interval = $1 WHERE id = $2",
            interval,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(easy_days) = &body.easy_days {
        sqlx::query!(
            "UPDATE deck_options SET easy_days = $1 WHERE id = $2",
            serde_json::to_value(easy_days).unwrap_or(serde_json::Value::Null),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(order) = body.insertion_order {
        sqlx::query!(
            "UPDATE deck_options SET insertion_order = $1::text::insertion_order WHERE id = $2",
            DbInsertionOrder::from(order).as_str(),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

        // Retroactive re-sort of existing new cards in this preset's decks
        // (US-2.13). Scoped to the preset (a documented divergence from Anki's
        // global position namespace). `random` shuffles; `sequential` restores
        // monotonic creation order. Renumbering all cards in the deck is
        // harmless — `position` only affects new-card ordering.
        let reorder = if order == anjuman_contracts::deck_options::InsertionOrder::Random {
            "floor(random() * 100000000)::bigint"
        } else {
            "c.id"
        };
        sqlx::query(&format!(
            "UPDATE cards c SET position = {reorder} FROM decks d JOIN deck_options o ON o.id = COALESCE(d.options_id, 0) WHERE c.deck_id = d.id AND o.id = $1"
        ))
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
    }
    if let Some(params) = &body.fsrs_parameters {
        sqlx::query!(
            "UPDATE deck_options SET fsrs_parameters = $1 WHERE id = $2",
            serde_json::to_value(params).unwrap_or(serde_json::Value::Null),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;
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
