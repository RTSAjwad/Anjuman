// Review handler.
//
// A student submits a rating (1-4) for a card they just reviewed.
// The FSRS algorithm calculates the next review interval and updates
// the student's scheduling state. A review record is created for
// analytics and future parameter optimisation.
//
// ## Ratings
//
//  1 — Again (failed, show again soon)
//  2 — Hard  (recalled with significant difficulty)
//  3 — Good  (recalled with acceptable effort)
//  4 — Easy  (recalled effortlessly)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use chrono::{DateTime, Duration, Utc};

use anjuman_contracts::reviews::{
    FlagResponse, ReviewResponse, ReviewedCardState, SetFlag, SubmitReview,
};

use crate::{auth::AuthUser, db_types::DbCardState, state::AppState};

// ---------------------------------------------------------------------------
// Learning & relearning step parsing
// ---------------------------------------------------------------------------

/// Parse an Anki-style step string into seconds.
///
/// Supports units:
///   - `s` = seconds, `m` = minutes, `h` = hours, `d` = days
///   - bare numbers default to minutes (Anki convention)
///   - decimals are allowed (e.g. `1.5d`)
///
/// Example: `"1m 1d"` → `[60, 86400]`.
#[allow(dead_code)] // used once a config endpoint is added
pub fn parse_steps(input: &str) -> Result<Vec<i64>, String> {
    let mut steps = Vec::new();
    for token in input.split_whitespace() {
        // Find the split point between the numeric part and the unit suffix.
        let split_at = token
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(token.len());
        let (num_str, unit) = token.split_at(split_at);

        let value: f64 = num_str
            .parse()
            .map_err(|_| format!("Invalid step value: '{token}'"))?;

        let multiplier: f64 = match unit {
            "" | "m" => 60.0,
            "s" => 1.0,
            "h" => 3600.0,
            "d" => 86400.0,
            other => return Err(format!("Unknown step unit: '{other}'")),
        };

        let seconds = (value * multiplier).round() as i64;
        if seconds <= 0 {
            return Err(format!("Step must be positive: '{token}'"));
        }
        steps.push(seconds);
    }

    if steps.is_empty() {
        return Err("At least one step is required".to_string());
    }

    Ok(steps)
}

// ---------------------------------------------------------------------------
// Sibling burying
// ---------------------------------------------------------------------------

/// A sibling candidate row (another card of the answered card's note).
struct SiblingRow {
    card_id: i64,
    state: DbCardState,
    step_index: i64,
    suspended: bool,
    buried_at: Option<DateTime<Utc>>,
    deck_id: i64,
}

/// Classify a card into an Anki gathering-order rank.
///
/// 0 = intraday learning, 1 = interday learning, 2 = review, 3 = new.
fn queue_class_rank(
    state: DbCardState,
    step_index: i64,
    learning_steps: &[i64],
    relearning_steps: &[i64],
) -> i64 {
    match state {
        DbCardState::Learning => {
            if is_interday_step(step_index, learning_steps) {
                1
            } else {
                0
            }
        }
        DbCardState::Relearning => {
            if is_interday_step(step_index, relearning_steps) {
                1
            } else {
                0
            }
        }
        DbCardState::Review => 2,
        _ => 3, // new
    }
}

fn is_interday_step(step_index: i64, steps: &[i64]) -> bool {
    steps.get(step_index as usize).is_some_and(|s| *s >= 86400)
}

/// The Hard-button delay (seconds) for a card on a (re)learning step,
/// following Anki's rules:
///
/// - **First step** → the average of the first two steps.
/// - **Single step** → 1.5× that step, capped at one day longer than the step.
/// - **Any other step** → the current step's delay (i.e. Hard repeats the step).
///
/// See <https://docs.ankiweb.net/deck-options.html#learning-steps>.
pub fn hard_step_delay(steps: &[i64], step_index: i64) -> i64 {
    match steps.len() {
        0 => 0,
        1 => {
            // 1.5× the single step, at most 1 day longer than the step.
            let step = steps[0];
            ((step as f64 * 1.5).round() as i64).min(step + 86400)
        }
        _ if step_index == 0 => {
            // Average of the first two steps.
            ((steps[0] + steps[1]) as f64 / 2.0).round() as i64
        }
        _ => steps.get(step_index as usize).copied().unwrap_or(0),
    }
}

/// Bury siblings of the answered card (same note), following Anki's
/// directional gathering-order rule.
///
/// A sibling is buried iff its own queue class is later-or-equal to the
/// answered card's class AND the corresponding toggle (from the answered
/// card's deck preset) is enabled. Intraday learning is never buried.
#[allow(clippy::too_many_arguments)]
async fn bury_siblings(
    db: &sqlx::PgPool,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    student_id: i64,
    note_id: i64,
    answered_card_id: i64,
    answered_rank: i64,
    toggles: &crate::deck_options::DeckOptions,
    now: DateTime<Utc>,
) -> Result<(), StatusCode> {
    // Read sibling candidates from within the same transaction so the
    // read→write decision is made against a consistent snapshot.
    let siblings = sqlx::query_as!(
        SiblingRow,
        r#"
        SELECT scs.card_id, scs.state as "state!: DbCardState", scs.step_index as "step_index: i64",
               scs.suspended as "suspended: bool", scs.buried_at, c.deck_id
        FROM student_card_states scs
        JOIN cards c ON c.id = scs.card_id
        WHERE scs.student_id = $1 AND c.note_id = $2 AND scs.card_id != $3
        "#,
        student_id,
        note_id,
        answered_card_id
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for s in siblings {
        if s.suspended || s.buried_at.is_some() {
            continue;
        }

        // Classify using the sibling's own deck steps (read-only; pool is fine).
        let opts = crate::deck_options::options_for_deck(db, s.deck_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let rank = queue_class_rank(
            s.state,
            s.step_index,
            &opts.learning_steps,
            &opts.relearning_steps,
        );

        // Intraday learning (0) is never buried; earlier-priority siblings
        // (lower rank) can't be buried by a later-priority answered card.
        if rank == 0 || rank < answered_rank {
            continue;
        }

        let (enabled, reason) = match rank {
            1 => (toggles.bury_interday, "sibling_interday"),
            2 => (toggles.bury_review, "sibling_review"),
            _ => (toggles.bury_new, "sibling_new"),
        };

        if !enabled {
            continue;
        }

        sqlx::query!(
            "UPDATE student_card_states SET buried_at = $1, bury_reason = $2 WHERE student_id = $3 AND card_id = $4",
            now,
            reason,
            student_id,
            s.card_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Apply a review to a card and return its post-review scheduling state.
///
/// This is the single authority for the FSRS + learning/relearning step logic.
/// It writes the updated `student_card_states` row and inserts a `reviews`
/// record (including `state_before` for daily-limit accounting).
///
/// `card_id` is validated against the owning deck's visibility by the caller.
pub async fn apply_review(
    db: &sqlx::PgPool,
    student_id: i64,
    card_id: i64,
    rating: i32,
    response_time_ms: Option<i64>,
) -> Result<ReviewedCardState, (StatusCode, &'static str)> {
    if !(1..=4).contains(&rating) {
        return Err((StatusCode::BAD_REQUEST, "Rating must be between 1 and 4"));
    }

    // Fetch the current scheduling state (and the card's deck + note).
    let current = sqlx::query!(
        r#"
        SELECT scs.state as "state!: DbCardState", scs.stability, scs.difficulty, scs.last_reviewed_at,
               scs.reps, scs.lapses, scs.step_index, c.deck_id, c.note_id
        FROM student_card_states scs
        JOIN cards c ON c.id = scs.card_id
        WHERE scs.student_id = $1 AND scs.card_id = $2
        "#,
        student_id,
        card_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Card state not found"))?;

    // Resolve this deck's effective scheduling options (preset or defaults).
    let options = crate::deck_options::options_for_deck(db, current.deck_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load deck options",
            )
        })?;
    let learning_steps = &options.learning_steps;
    let relearning_steps = &options.relearning_steps;
    let desired_retention = options.desired_retention;

    // The answered card's queue class (pre-review), used to decide which
    // siblings it may bury.
    let answered_rank = queue_class_rank(
        current.state,
        current.step_index,
        learning_steps,
        relearning_steps,
    );

    let now = Utc::now();

    // Calculate elapsed days since last review.
    let elapsed_days = if let Some(last_reviewed) = current.last_reviewed_at {
        (now - last_reviewed).num_days().max(0) as u32
    } else {
        0
    };

    // Build the previous memory state for FSRS.
    let previous_memory = if current.reps > 0 {
        Some(fsrs::MemoryState {
            stability: current.stability as f32,
            difficulty: current.difficulty as f32,
        })
    } else {
        None
    };

    // Run the FSRS scheduler.
    let fsrs = fsrs::FSRS::default();

    let next_states = fsrs
        .next_states(previous_memory, desired_retention as f32, elapsed_days)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "FSRS scheduling failed"))?;

    // Choose the output based on the rating.
    let next = match rating {
        1 => next_states.again,
        2 => next_states.hard,
        3 => next_states.good,
        4 => next_states.easy,
        _ => unreachable!(),
    };

    let interval_days = next.interval.round().max(1.0) as i64;
    let interval_fsrs_secs = interval_days * 86400;
    // `interval_days`/`interval_fsrs_secs` are the FSRS *graduation* interval,
    // used only when the card transitions to 'review'. Step outcomes below
    // ignore these in favour of explicit learning/relearning step intervals.

    // Determine the new state, step index, and due timestamp.
    //
    // Learning/relearning cards follow Anki's step model:
    //   - Steps are fixed intervals (in seconds).
    //   - "Good" advances one step; graduating past the last step → review.
    //   - "Easy" graduates immediately.
    //   - "Again" resets to the first step.
    //   - "Hard" advances one step like "Good" (FSRS still records lower
    //     stability, affecting future intervals after graduation).
    // Review cards lapse back to relearning on "Again".
    let mut step_index = current.step_index;
    let new_state: DbCardState;
    let due_at: DateTime<Utc>;

    match current.state {
        DbCardState::New => match rating {
            4 => {
                // Easy: graduate immediately.
                new_state = DbCardState::Review;
                step_index = 0;
                due_at = now + Duration::seconds(interval_fsrs_secs);
            }
            3 => {
                // Good: advance one step; graduate if past last.
                step_index += 1;
                if step_index >= learning_steps.len() as i64 {
                    new_state = DbCardState::Review;
                    step_index = 0;
                    due_at = now + Duration::seconds(interval_fsrs_secs);
                } else {
                    new_state = DbCardState::Learning;
                    due_at = now + Duration::seconds(learning_steps[step_index as usize]);
                }
            }
            2 => {
                // Hard on the first step: average of the first two steps. Hard
                // does not advance the step count, so the card stays put.
                new_state = DbCardState::Learning;
                step_index = 0;
                due_at = now + Duration::seconds(hard_step_delay(learning_steps, 0));
            }
            _ => {
                // Again: stay in learning at first step.
                new_state = DbCardState::Learning;
                step_index = 0;
                due_at = now + Duration::seconds(learning_steps[0]);
            }
        },
        DbCardState::Learning => match rating {
            1 => {
                // Again: reset to first step.
                step_index = 0;
                due_at = now + Duration::seconds(learning_steps[0]);
                new_state = DbCardState::Learning;
            }
            2 => {
                // Hard: repeats the current step (does not advance).
                new_state = DbCardState::Learning;
                due_at =
                    now + Duration::seconds(hard_step_delay(learning_steps, step_index));
            }
            3 => {
                // Good: advance one step; graduate if past last.
                step_index += 1;
                if step_index >= learning_steps.len() as i64 {
                    new_state = DbCardState::Review;
                    step_index = 0;
                    due_at = now + Duration::seconds(interval_fsrs_secs);
                } else {
                    new_state = DbCardState::Learning;
                    due_at = now + Duration::seconds(learning_steps[step_index as usize]);
                }
            }
            _ => {
                // Easy: graduate immediately.
                new_state = DbCardState::Review;
                step_index = 0;
                due_at = now + Duration::seconds(interval_fsrs_secs);
            }
        },
        DbCardState::Review => match rating {
            1 => {
                // Again: lapse to relearning.
                new_state = DbCardState::Relearning;
                step_index = 0;
                due_at = now + Duration::seconds(relearning_steps[0]);
            }
            _ => {
                // Hard/Good/Easy: stay review.
                new_state = DbCardState::Review;
                due_at = now + Duration::seconds(interval_fsrs_secs);
            }
        },
        DbCardState::Relearning => match rating {
            1 => {
                // Again: restart relearning steps.
                step_index = 0;
                due_at = now + Duration::seconds(relearning_steps[0]);
                new_state = DbCardState::Relearning;
            }
            2 => {
                // Hard: repeats the current step (does not advance).
                new_state = DbCardState::Relearning;
                due_at =
                    now + Duration::seconds(hard_step_delay(relearning_steps, step_index));
            }
            3 => {
                // Good: advance one step; graduate if past last.
                step_index += 1;
                if step_index >= relearning_steps.len() as i64 {
                    new_state = DbCardState::Review;
                    step_index = 0;
                    due_at = now + Duration::seconds(interval_fsrs_secs);
                } else {
                    new_state = DbCardState::Relearning;
                    due_at = now + Duration::seconds(relearning_steps[step_index as usize]);
                }
            }
            _ => {
                // Easy: graduate immediately.
                new_state = DbCardState::Review;
                step_index = 0;
                due_at = now + Duration::seconds(interval_fsrs_secs);
            }
        },
    }

    // A lapse, in Anki's leech sense, is a review (graduated) card answered
    // "Again" — not a learning/relearning step reset. Leech counting uses this.
    let is_review_lapse = current.state == DbCardState::Review && rating == 1;

    let new_reps = current.reps + 1;
    let new_lapses = if is_review_lapse {
        current.lapses + 1
    } else {
        current.lapses
    };

    // Leech detection: when a review card's lapse count reaches the preset's
    // threshold, suspend it (and, for SuspendCard, mark the note as a leech).
    // Tag-only is a no-op until the tags system lands (ROADMAP stage 6).
    let reached_leech = is_review_lapse && new_lapses >= options.leech_threshold;

    // Wrap the review's writes (scheduling state, review record, and any
    // sibling buries) in a single IMMEDIATE transaction so the action is
    // atomic: either all of it lands or none of it does.
    let mut tx = crate::db::begin_immediate(db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Update the scheduling state.
    sqlx::query!(
        r#"
        UPDATE student_card_states
        SET state = $1::text::card_state, stability = $2, difficulty = $3,
            step_index = $4, due_at = $5, last_reviewed_at = $6, reps = $7, lapses = $8
        WHERE student_id = $9 AND card_id = $10
        "#,
        new_state.as_str(),
        next.memory.stability as f64,
        next.memory.difficulty as f64,
        step_index,
        due_at,
        now,
        new_reps,
        new_lapses,
        student_id,
        card_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Record the review (with the pre-review state for daily-limit accounting).
    sqlx::query!(
        "INSERT INTO reviews (student_id, card_id, rating, reviewed_at, response_time_ms, state_before) VALUES ($1, $2, $3, $4, $5, $6)",
        student_id,
        card_id,
        rating as i64,
        now,
        response_time_ms,
        current.state.as_str()
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Leech handling: suspend the card at the threshold, and mark the note.
    if reached_leech {
        if options.leech_action == anjuman_contracts::deck_options::LeechAction::SuspendCard {
            sqlx::query!(
                "UPDATE student_card_states SET suspended = TRUE WHERE student_id = $1 AND card_id = $2",
                student_id,
                card_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }

        // Mark the note as a leech (minimal marker; a full tag system replaces
        // this in ROADMAP stage 6).
        sqlx::query!(
            "UPDATE notes SET leech_tagged_at = $1 WHERE id = $2",
            now,
            current.note_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    // Bury siblings (same note) as appropriate, per the deck's sibling-bury
    // toggles and Anki's directional gathering-order rule.
    bury_siblings(
        db,
        &mut tx,
        student_id,
        current.note_id,
        card_id,
        answered_rank,
        &options,
        now,
    )
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(ReviewedCardState {
        card_id,
        state: new_state.as_str().to_string(),
        due_at: Some(due_at),
        stability: next.memory.stability as f64,
        difficulty: next.memory.difficulty as f64,
        reps: new_reps,
        lapses: new_lapses,
        step_index,
        applied_interval_secs: (due_at - now).num_seconds().max(0),
    })
}

/// `POST /reviews` — Submit a card review rating.
pub async fn submit_review(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SubmitReview>,
) -> Result<Json<ReviewResponse>, (StatusCode, &'static str)> {
    let reviewed = apply_review(
        &state.db,
        claims.sub,
        body.card_id,
        body.rating,
        body.response_time_ms,
    )
    .await?;

    Ok(Json(ReviewResponse {
        card_id: reviewed.card_id,
        state: reviewed.state,
        due_at: reviewed.due_at,
        stability: reviewed.stability,
        difficulty: reviewed.difficulty,
        reps: reviewed.reps,
        lapses: reviewed.lapses,
        applied_interval_secs: reviewed.applied_interval_secs,
    }))
}

/// `PATCH /cards/:card_id/flag` — Set or clear a flag on a card.
///
/// Flags are per-student, per-card markers (Anki-style).
/// Values: 0 (none), 1 (red), 2 (orange), 3 (green), 4 (blue),
/// 5 (pink), 6 (turquoise), 7 (purple).
pub async fn set_flag(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
    Json(body): Json<SetFlag>,
) -> Result<Json<FlagResponse>, (StatusCode, &'static str)> {
    if !(0..=7).contains(&body.flag) {
        return Err((StatusCode::BAD_REQUEST, "Flag must be between 0 and 7"));
    }

    // Ensure a state row exists so flags work on never-studied cards too.
    sqlx::query!(
        "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0) ON CONFLICT (student_id, card_id) DO NOTHING",
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let result = sqlx::query!(
        "UPDATE student_card_states SET flag = $1 WHERE student_id = $2 AND card_id = $3",
        body.flag as i64,
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card not found"));
    }

    Ok(Json(FlagResponse {
        card_id,
        flag: body.flag as i64,
    }))
}

#[cfg(test)]
mod tests {
    use super::hard_step_delay;

    #[test]
    fn hard_first_step_is_average_of_first_two() {
        // Default steps "1m 10m" → Hard on the first step = (60 + 600)/2 = 330s
        // (5m30s). The Anki manual calls this "6m" (display-rounded); the real
        // client shows "<6m". We keep the exact seconds. See
        // DECK_OPTIONS_SUPPORT.md "Hard-button delay precision".
        assert_eq!(hard_step_delay(&[60, 600, 86400], 0), 330);
    }

    #[test]
    fn hard_single_step_is_one_point_five_times() {
        // 1.5× the single step.
        assert_eq!(hard_step_delay(&[60], 0), 90);
    }

    #[test]
    fn hard_single_step_is_capped_at_one_day_longer() {
        // A 3-day single step: 1.5× = 4.5 days, but capped at 3d + 1d = 4 days.
        let three_days = 3 * 86400;
        assert_eq!(hard_step_delay(&[three_days], 0), three_days + 86400);
    }

    #[test]
    fn hard_other_step_repeats_current_step() {
        // On the second step of "1m 10m", Hard repeats the second step (10m).
        assert_eq!(hard_step_delay(&[60, 600], 1), 600);
    }

    #[test]
    fn hard_empty_steps_is_zero() {
        assert_eq!(hard_step_delay(&[], 0), 0);
    }
}
