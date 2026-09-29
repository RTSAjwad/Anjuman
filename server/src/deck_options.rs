// Deck options presets and resolution.
//
// Mirrors Anki's deck options ("dconf") model: decks reference a named preset
// via `options_id`. A deck with no preset falls back to the global default
// preset (owned by the system school, id 0).
//
// Scheduling steps (learning/relearning) are stored normalised in
// `deck_option_steps` (one row per step) and re-assembled into `Vec<i64>` here.

use sqlx::PgPool;

use crate::db_types::{
    DbAutoAdvanceAnswerAction, DbAutoAdvanceQuestionAction, DbInsertionOrder, DbInterdayOrder,
    DbLeechAction, DbLimitMode, DbNewGatherOrder, DbNewReviewOrder, DbNewSortOrder,
    DbReviewSortOrder, DbStepKind,
};

pub use anjuman_contracts::deck_options::DeckOptions;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// `DeckOptions` (the wire type) is re-exported from `anjuman_contracts`.

// ---------------------------------------------------------------------------
// Step persistence
// ---------------------------------------------------------------------------

/// Replace a preset's learning and relearning steps in `deck_option_steps`.
///
/// Deletes existing step rows for the preset and re-inserts the given lists
/// in order. `step_index` is derived from the vector position. Must be called
/// inside a caller's transaction so the delete+insert is atomic.
pub async fn replace_steps(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    options_id: i64,
    learning_steps: &[i64],
    relearning_steps: &[i64],
) -> Result<(), String> {
    sqlx::query!(
        "DELETE FROM deck_option_steps WHERE options_id = $1",
        options_id
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    for (i, secs) in learning_steps.iter().enumerate() {
        let step_index = i as i64;
        sqlx::query!(
            "INSERT INTO deck_option_steps (options_id, kind, step_index, seconds) VALUES ($1, 'learning', $2, $3)",
            options_id,
            step_index,
            secs
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    }

    for (i, secs) in relearning_steps.iter().enumerate() {
        let step_index = i as i64;
        sqlx::query!(
            "INSERT INTO deck_option_steps (options_id, kind, step_index, seconds) VALUES ($1, 'relearning', $2, $3)",
            options_id,
            step_index,
            secs
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Database access
// ---------------------------------------------------------------------------

/// Fetch a preset by id, including its normalised steps.
pub async fn get_options(db: &PgPool, id: i64) -> Result<DeckOptions, String> {
    let row = sqlx::query!(
        "SELECT id, school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, leech_threshold, leech_action as \"leech_action!: DbLeechAction\", new_gather_order as \"new_gather_order!: DbNewGatherOrder\", new_sort_order as \"new_sort_order!: DbNewSortOrder\", new_review_order as \"new_review_order!: DbNewReviewOrder\", interday_order as \"interday_order!: DbInterdayOrder\", review_sort_order as \"review_sort_order!: DbReviewSortOrder\", show_on_screen_timer, stop_timer_on_answer, dont_play_audio_automatically, skip_question_when_replaying_answer, auto_advance_seconds_show_question, auto_advance_seconds_show_answer, auto_advance_wait_for_audio, auto_advance_question_action as \"auto_advance_question_action!: DbAutoAdvanceQuestionAction\", auto_advance_answer_action as \"auto_advance_answer_action!: DbAutoAdvanceAnswerAction\", maximum_answer_seconds, maximum_interval, easy_days, insertion_order as \"insertion_order!: DbInsertionOrder\", fsrs_parameters FROM deck_options WHERE id = $1",
        id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| format!("Deck options {id} not found"))?;

    let steps = load_steps(db, id).await?;

    Ok(DeckOptions {
        id: row.id,
        school_id: row.school_id,
        name: row.name,
        learning_steps: steps.learning_steps,
        relearning_steps: steps.relearning_steps,
        desired_retention: row.desired_retention,
        bury_new: row.bury_new,
        bury_review: row.bury_review,
        bury_interday: row.bury_interday,
        new_per_day: row.new_per_day,
        review_per_day: row.review_per_day,
        leech_threshold: row.leech_threshold,
        leech_action: row.leech_action.into(),
        new_gather_order: row.new_gather_order.into(),
        new_sort_order: row.new_sort_order.into(),
        new_review_order: row.new_review_order.into(),
        interday_order: row.interday_order.into(),
        review_sort_order: row.review_sort_order.into(),
        show_on_screen_timer: row.show_on_screen_timer,
        stop_timer_on_answer: row.stop_timer_on_answer,
        dont_play_audio_automatically: row.dont_play_audio_automatically,
        skip_question_when_replaying_answer: row.skip_question_when_replaying_answer,
        auto_advance_seconds_show_question: row.auto_advance_seconds_show_question,
        auto_advance_seconds_show_answer: row.auto_advance_seconds_show_answer,
        auto_advance_wait_for_audio: row.auto_advance_wait_for_audio,
        auto_advance_question_action: row.auto_advance_question_action.into(),
        auto_advance_answer_action: row.auto_advance_answer_action.into(),
        maximum_answer_seconds: row.maximum_answer_seconds,
        maximum_interval: row.maximum_interval,
        easy_days: serde_json::from_value(row.easy_days)
            .unwrap_or_else(|_| vec![anjuman_contracts::deck_options::EasyDayStrength::Normal; 7]),
        insertion_order: row.insertion_order.into(),
        fsrs_parameters: serde_json::from_value(row.fsrs_parameters).unwrap_or_default(),
    })
}

/// Load a preset's learning and relearning steps from `deck_option_steps`.
async fn load_steps(db: &PgPool, options_id: i64) -> Result<Steps, String> {
    let rows = sqlx::query!(
        "SELECT kind as \"kind!: DbStepKind\", step_index, seconds FROM deck_option_steps WHERE options_id = $1 ORDER BY kind, step_index",
        options_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    let mut learning_steps = Vec::new();
    let mut relearning_steps = Vec::new();
    for r in rows {
        match r.kind {
            DbStepKind::Learning => learning_steps.push(r.seconds),
            DbStepKind::Relearning => relearning_steps.push(r.seconds),
        }
    }

    Ok(Steps {
        learning_steps,
        relearning_steps,
    })
}

struct Steps {
    learning_steps: Vec<i64>,
    relearning_steps: Vec<i64>,
}

/// Resolve the effective options for a deck.
///
/// If the deck has an `options_id`, return that preset; otherwise return the
/// global default preset (owned by the system school, id 0).
pub async fn options_for_deck(db: &PgPool, deck_id: i64) -> Result<DeckOptions, String> {
    let options_id: Option<Option<i64>> =
        sqlx::query_scalar("SELECT options_id FROM decks WHERE id = $1")
            .bind(deck_id)
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| format!("Deck {deck_id} not found"))?;

    match options_id {
        Some(Some(id)) => get_options(db, id).await,
        // Fall back to the global default preset (id 0).
        _ => get_options(db, 0).await,
    }
}

/// The effective daily new/review limits for a deck, applying any per-deck
/// `preset`/`this_deck`/`today_only` override (US-2.6).
///
/// Returns `(new_per_day, review_per_day)`. A `today_only` override whose set
/// date is before `today` (the student's study-day start) expires and falls
/// back to the preset limit.
pub async fn effective_daily_limits(
    db: &PgPool,
    deck_id: i64,
    today: chrono::NaiveDate,
) -> Result<(i64, i64), String> {
    let preset = options_for_deck(db, deck_id).await?;

    let row = sqlx::query!(
        r#"
        SELECT new_per_day_mode as "new_per_day_mode!: DbLimitMode",
               review_per_day_mode as "review_per_day_mode!: DbLimitMode",
               new_per_day_override,
               review_per_day_override,
               new_per_day_today_date,
               review_per_day_today_date
        FROM decks
        WHERE id = $1
        "#,
        deck_id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| format!("Deck {deck_id} not found"))?;

    let new_limit = resolve_limit(
        row.new_per_day_mode,
        row.new_per_day_override,
        row.new_per_day_today_date,
        preset.new_per_day,
        today,
    );
    let review_limit = resolve_limit(
        row.review_per_day_mode,
        row.review_per_day_override,
        row.review_per_day_today_date,
        preset.review_per_day,
        today,
    );

    Ok((new_limit, review_limit))
}

/// Apply one deck's limit mode to produce its effective value.
fn resolve_limit(
    mode: DbLimitMode,
    override_value: Option<i64>,
    today_date: Option<chrono::NaiveDate>,
    preset_value: i64,
    today: chrono::NaiveDate,
) -> i64 {
    match mode {
        DbLimitMode::Preset => preset_value,
        DbLimitMode::ThisDeck => override_value.unwrap_or(preset_value),
        DbLimitMode::TodayOnly => {
            if today_date == Some(today) {
                override_value.unwrap_or(preset_value)
            } else {
                // Expired today-only override; fall back to preset.
                preset_value
            }
        }
    }
}
