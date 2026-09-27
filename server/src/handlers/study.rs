// Study flow.
//
// The study flow is single-card and stateless, mirroring Anki's scheduler:
//
//   GET /decks/:id/study  — start; returns the first due card plus counts.
//   POST /decks/:id/study — advance; submits a rating, then returns the next
//                           due card plus counts (and the reviewed card's new
//                           state).
//
// The client loops the POST endpoint until `next_card` is null (nothing is
// currently due), rather than holding a pre-fetched queue. This keeps the
// server from guessing what the client has already seen and naturally handles
// "Again" re-queueing (a card lapsed on a short step reappears once its
// `due_at` is reached).
//
// "Due now" is the single source of truth shared with GET /decks/counts and
// the student branch of GET /decks: not suspended, not buried, and in one of
// the studyable states at the current time. Daily limits (new_per_day /
// review_per_day) are enforced here and reflect into the returned counts.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use anjuman_contracts::study::{StudyAdvance, StudyAdvanceBody, StudyCard, StudyCounts};

use crate::{
    auth::AuthUser,
    handlers::decks,
    note_types,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Shared scheduling helpers
// ---------------------------------------------------------------------------

/// A card row for study, carrying the fields needed to render and schedule.
struct CardRow {
    id: Option<i64>,
    note_id: i64,
    template_id: i64,
    note_type_id: i64,
    fields_json: String,
    state: String,
    due_at: Option<i64>,
    stability: f64,
    difficulty: f64,
    reps: i64,
    lapses: i64,
    flag: i64,
    suspended: i64,
    buried_at: Option<i64>,
    bury_reason: Option<String>,
    step_index: i64,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Compute the start of the current "study day" as a UTC epoch, given a
/// day-start hour (0-23). The day is anchored to UTC for now (no timezone).
fn day_start_utc(now: i64, day_start_hour: i64) -> i64 {
    let seconds_past_midnight = now % 86400;
    let start_seconds = day_start_hour * 3600;
    if seconds_past_midnight >= start_seconds {
        now - (seconds_past_midnight - start_seconds)
    } else {
        now - (seconds_past_midnight + 86400 - start_seconds)
    }
}

/// Resolve the student's start-of-day hour (Anki's "Next day starts at",
/// default 4 AM). Falls back to 4 when no preference row exists.
async fn day_start_hour(db: &sqlx::PgPool, student_id: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT day_start_hour FROM user_preferences WHERE user_id = ?",
        student_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(4)
}

/// Start of the current study day for a student.
async fn start_of_day(db: &sqlx::PgPool, student_id: i64) -> i64 {
    let hour = day_start_hour(db, student_id).await;
    day_start_utc(now_secs(), hour)
}

/// Resolve the student's learn-ahead limit (in seconds).
///
/// This is a per-user option (Anki's "Learn ahead limit", default 20 minutes).
/// Falls back to 1200 when no preference row exists.
async fn learn_ahead_seconds(db: &sqlx::PgPool, student_id: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT learn_ahead_seconds FROM user_preferences WHERE user_id = ?",
        student_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(1200)
}

/// Create `student_card_states` rows for any card in `deck_id`'s subtree the
/// student has not seen yet (so `new` cards are studyable on first fetch).
async fn ensure_card_states_for_deck(
    db: &sqlx::PgPool,
    student_id: i64,
    deck_id: i64,
) -> Result<(), StatusCode> {
    sqlx::query!(
        r#"
        INSERT OR IGNORE INTO student_card_states
            (student_id, card_id, state, stability, difficulty, reps, lapses)
        SELECT ?, c.id, 'new', 0.0, 0.0, 0, 0
        FROM cards c
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT ?
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
        "#,
        student_id,
        deck_id
    )
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

/// The effective deck options (limits) for a deck.
async fn options_for(
    db: &sqlx::PgPool,
    deck_id: i64,
) -> Result<crate::deck_options::DeckOptions, StatusCode> {
    crate::deck_options::options_for_deck(db, deck_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// How many new / review cards the student has already seen today, derived
/// from the `reviews` table (and its `state_before` column). Returns
/// (new_seen_today, review_seen_today).
async fn seen_today(db: &sqlx::PgPool, student_id: i64) -> Result<(i64, i64), StatusCode> {
    let day_start = start_of_day(db, student_id).await;
    let row = sqlx::query!(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN state_before = 'new' THEN 1 ELSE 0 END), 0) as "new_seen!: i64",
            COALESCE(SUM(CASE WHEN state_before IN ('review', 'relearning') THEN 1 ELSE 0 END), 0) as "review_seen!: i64"
        FROM reviews
        WHERE student_id = ? AND reviewed_at >= ?
        "#,
        student_id,
        day_start
    )
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((row.new_seen, row.review_seen))
}

/// Limit-aware, due-now per-state counts for a student's deck subtree.
///
/// This is the single source of truth shared by the study flow and
/// GET /decks/counts (and the student branch of GET /decks). The physical
/// due-now counts are computed, then `new` and `review` are clamped to the
/// remaining daily budget.
pub async fn deck_counts_for_student(
    db: &sqlx::PgPool,
    student_id: i64,
    deck_id: i64,
) -> Result<StudyCounts, StatusCode> {
    let options = options_for(db, deck_id).await?;
    let (new_seen, review_seen) = seen_today(db, student_id).await?;
    let learn_ahead = learn_ahead_seconds(db, student_id).await;
    let day_start = start_of_day(db, student_id).await;

    let row = sqlx::query!(
        r#"
        WITH RECURSIVE subtree(id) AS (
            SELECT ?
            UNION ALL
            SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
        )
        SELECT
            COALESCE(SUM(CASE WHEN (scs.state = 'new') AND (scs.suspended = 0 AND (scs.buried_at IS NULL OR scs.buried_at < ?)) THEN 1 ELSE 0 END), 0) as "new_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'learning' AND scs.due_at <= unixepoch() + ? AND (scs.suspended = 0 AND (scs.buried_at IS NULL OR scs.buried_at < ?)) THEN 1 ELSE 0 END), 0) as "learning_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'review' AND scs.due_at <= unixepoch() AND (scs.suspended = 0 AND (scs.buried_at IS NULL OR scs.buried_at < ?)) THEN 1 ELSE 0 END), 0) as "review_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'relearning' AND scs.due_at <= unixepoch() + ? AND (scs.suspended = 0 AND (scs.buried_at IS NULL OR scs.buried_at < ?)) THEN 1 ELSE 0 END), 0) as "relearning_total!: i64"
        FROM cards c
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = ?
        WHERE c.deck_id IN (SELECT id FROM subtree)
        "#,
        deck_id,
        day_start,
        learn_ahead,
        day_start,
        day_start,
        learn_ahead,
        day_start,
        student_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new_remaining = (options.new_per_day - new_seen).max(0);
    let review_remaining = (options.review_per_day - review_seen).max(0);

    Ok(StudyCounts {
        new_count: row.new_total.min(new_remaining),
        learning_count: row.learning_total,
        review_count: row.review_total.min(review_remaining),
        relearning_count: row.relearning_total,
    })
}

/// Select the single highest-priority due card for a student's deck, respecting
/// daily limits and the learner's learn-ahead limit. Returns None when
/// nothing is currently due (or budgeted out, or outside the learn-ahead
/// window).
///
/// Cards are ranked by Anki's gathering order, computed entirely in SQL via a
/// join to `deck_option_steps` (intraday learning → interday learning → review
/// → new).
async fn next_due_card(
    db: &sqlx::PgPool,
    student_id: i64,
    deck_id: i64,
) -> Result<Option<CardRow>, StatusCode> {
    let options = options_for(db, deck_id).await?;
    let (new_seen, review_seen) = seen_today(db, student_id).await?;
    let learn_ahead = learn_ahead_seconds(db, student_id).await;
    let day_start = start_of_day(db, student_id).await;

    let new_remaining = (options.new_per_day - new_seen).max(0);
    let review_remaining = (options.review_per_day - review_seen).max(0);

    // Single query: rank due candidates by gathering order using the
    // normalised step table. Each card uses its own deck's preset.
    let row = sqlx::query_as!(
        CardRow,
        r#"
        SELECT c.id, c.note_id, c.template_id, n.note_type_id, n.fields_json,
               scs.state, scs.due_at, scs.stability as "stability: f64",
               scs.difficulty as "difficulty: f64", scs.reps, scs.lapses,
               scs.flag as "flag: i64", scs.suspended as "suspended: i64",
               scs.buried_at, scs.bury_reason, scs.step_index as "step_index: i64"
        FROM cards c
        JOIN notes n ON n.id = c.note_id
        JOIN decks cd ON cd.id = c.deck_id
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = ?
        LEFT JOIN deck_option_steps dos
            ON dos.options_id = COALESCE(cd.options_id, 0)
           AND dos.kind = CASE scs.state WHEN 'relearning' THEN 'relearning' ELSE 'learning' END
           AND dos.step_index = scs.step_index
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT ?
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
          AND scs.suspended = 0
          AND (scs.buried_at IS NULL OR scs.buried_at < ?)
          AND (
                (scs.state IN ('learning', 'relearning') AND scs.due_at <= unixepoch())
             OR (scs.state = 'review' AND scs.due_at <= unixepoch() AND ? > 0)
             OR (scs.state = 'new' AND ? > 0)
          )
        ORDER BY
            CASE
                WHEN scs.state IN ('learning', 'relearning') AND COALESCE(dos.seconds, 0) >= 86400 THEN 1
                WHEN scs.state IN ('learning', 'relearning') THEN 0
                WHEN scs.state = 'review' THEN 2
                ELSE 3
            END,
            scs.due_at ASC NULLS LAST,
            c.id ASC
        LIMIT 1
        "#,
        student_id,
        deck_id,
        day_start,
        review_remaining,
        new_remaining
    )
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if row.is_some() {
        return Ok(row);
    }

    // Learn-ahead fallback: no card is actually due, but a learning/relearning
    // card is due within the learn-ahead window. Show the soonest one.
    let ahead = sqlx::query_as!(
        CardRow,
        r#"
        SELECT c.id, c.note_id, c.template_id, n.note_type_id, n.fields_json,
               scs.state, scs.due_at, scs.stability as "stability: f64",
               scs.difficulty as "difficulty: f64", scs.reps, scs.lapses,
               scs.flag as "flag: i64", scs.suspended as "suspended: i64",
               scs.buried_at, scs.bury_reason, scs.step_index as "step_index: i64"
        FROM cards c
        JOIN notes n ON n.id = c.note_id
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = ?
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT ?
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
          AND scs.suspended = 0
          AND (scs.buried_at IS NULL OR scs.buried_at < ?)
          AND scs.state IN ('learning', 'relearning')
          AND scs.due_at > unixepoch()
          AND scs.due_at <= unixepoch() + ?
        ORDER BY scs.due_at ASC
        LIMIT 1
        "#,
        student_id,
        deck_id,
        day_start,
        learn_ahead
    )
    .fetch_optional(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(ahead)
}

/// Render a single card row into a full `StudyCard`, including predicted
/// intervals (in seconds) for each possible rating.
async fn row_to_study_card(
    db: &sqlx::PgPool,
    c: CardRow,
    options: &crate::deck_options::DeckOptions,
) -> Result<StudyCard, StatusCode> {
    let nt = note_types::get_note_type(db, c.note_type_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let fields: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&c.fields_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rendered = note_types::render_card(&nt.templates, c.template_id, &fields)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Predicted intervals (in seconds) for each rating, mirroring the exact
    // scheduling rules in `apply_review` so the pre-submission hints are
    // truthful.
    let predicted_interval = predict_intervals(&c, options);

    Ok(StudyCard {
        card_id: c.id.expect("card.id is NOT NULL"),
        note_id: c.note_id,
        front: rendered.front,
        back: rendered.back,
        state: c.state,
        due_at: c.due_at,
        stability: c.stability,
        difficulty: c.difficulty,
        reps: c.reps,
        lapses: c.lapses,
        flag: c.flag,
        suspended: c.suspended,
        buried_at: c.buried_at,
        bury_reason: c.bury_reason,
        step_index: c.step_index,
        predicted_interval,
    })
}

/// Compute the predicted next interval (in seconds) for each rating, using the
/// same step + FSRS graduation rules applied by `apply_review`. This makes the
/// frontend's interval hints consistent with the actual scheduling outcome.
fn predict_intervals(
    c: &CardRow,
    options: &crate::deck_options::DeckOptions,
) -> Option<HashMap<String, i64>> {
    let fsrs = fsrs::FSRS::default();
    let desired_retention = options.desired_retention as f32;
    let now = now_secs();

    // FSRS memory-state intervals (in days) for graduation, converted to secs.
    let fsrs_intervals = {
        let previous = if c.reps > 0 {
            Some(fsrs::MemoryState {
                stability: c.stability as f32,
                difficulty: c.difficulty as f32,
            })
        } else {
            None
        };
        let elapsed = c
            .due_at
            .map(|due| ((now - due).max(0) as f64 / 86400.0) as u32)
            .unwrap_or(0);
        fsrs.next_states(previous, desired_retention, elapsed)
            .ok()
            .map(|next| {
                [
                    next.again.interval,
                    next.hard.interval,
                    next.good.interval,
                    next.easy.interval,
                ]
            })
    };

    let learning_steps = &options.learning_steps;
    let relearning_steps = &options.relearning_steps;
    let interval_secs = |days: f32| (days.max(1.0).round() as i64) * 86400;

    let mut map = HashMap::new();

    match c.state.as_str() {
        "new" => {
            // Again/Hard: first learning step; Good: next step; Easy: graduate.
            map.insert(
                "1".to_string(),
                learning_steps.first().copied().unwrap_or(60),
            );
            map.insert(
                "2".to_string(),
                learning_steps.first().copied().unwrap_or(60),
            );
            let good = if learning_steps.len() <= 1 {
                fsrs_intervals
                    .as_ref()
                    .map(|f| interval_secs(f[2]))
                    .unwrap_or(86400)
            } else {
                learning_steps.get(1).copied().unwrap_or(600)
            };
            map.insert("3".to_string(), good);
            let easy = fsrs_intervals
                .as_ref()
                .map(|f| interval_secs(f[3]))
                .unwrap_or(86400 * 4);
            map.insert("4".to_string(), easy);
        }
        "learning" => {
            // Again: first step; Hard/Good: next step (or graduate); Easy: graduate.
            map.insert(
                "1".to_string(),
                learning_steps.first().copied().unwrap_or(60),
            );
            let next_idx = (c.step_index as usize + 1).min(learning_steps.len());
            let hard_good = if next_idx >= learning_steps.len() {
                fsrs_intervals
                    .as_ref()
                    .map(|f| interval_secs(f[2]))
                    .unwrap_or(86400)
            } else {
                learning_steps.get(next_idx).copied().unwrap_or(600)
            };
            map.insert("2".to_string(), hard_good);
            map.insert("3".to_string(), hard_good);
            let easy = fsrs_intervals
                .as_ref()
                .map(|f| interval_secs(f[3]))
                .unwrap_or(86400 * 4);
            map.insert("4".to_string(), easy);
        }
        "review" => {
            // Again: first relearning step; Hard/Good/Easy: FSRS intervals.
            map.insert(
                "1".to_string(),
                relearning_steps.first().copied().unwrap_or(600),
            );
            if let Some(f) = &fsrs_intervals {
                map.insert("2".to_string(), interval_secs(f[1]));
                map.insert("3".to_string(), interval_secs(f[2]));
                map.insert("4".to_string(), interval_secs(f[3]));
            } else {
                map.insert("2".to_string(), 86400);
                map.insert("3".to_string(), 86400);
                map.insert("4".to_string(), 86400 * 4);
            }
        }
        "relearning" => {
            // Again: first relearning step; Hard/Good: next step (or graduate).
            map.insert(
                "1".to_string(),
                relearning_steps.first().copied().unwrap_or(600),
            );
            let next_idx = (c.step_index as usize + 1).min(relearning_steps.len());
            let hard_good = if next_idx >= relearning_steps.len() {
                fsrs_intervals
                    .as_ref()
                    .map(|f| interval_secs(f[2]))
                    .unwrap_or(86400)
            } else {
                relearning_steps.get(next_idx).copied().unwrap_or(600)
            };
            map.insert("2".to_string(), hard_good);
            map.insert("3".to_string(), hard_good);
            let easy = fsrs_intervals
                .as_ref()
                .map(|f| interval_secs(f[3]))
                .unwrap_or(86400 * 4);
            map.insert("4".to_string(), easy);
        }
        _ => {
            return None;
        }
    }

    Some(map)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /decks/:id/study` — Start studying a deck; return the first due card.
pub async fn deck_study(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
) -> Result<Json<StudyAdvance>, (StatusCode, &'static str)> {
    deck_advance(&state.db, &claims, deck_id, None).await
}

/// `POST /decks/:id/study` — Submit a rating and advance to the next card.
pub async fn deck_study_advance(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Json(body): Json<StudyAdvanceBody>,
) -> Result<Json<StudyAdvance>, (StatusCode, &'static str)> {
    if !(1..=4).contains(&body.rating) {
        return Err((StatusCode::BAD_REQUEST, "Rating must be between 1 and 4"));
    }
    let _ = deck_id; // validated inside deck_advance; card ownership checked there too
    deck_advance(&state.db, &claims, deck_id, Some(body)).await
}

/// Shared implementation for GET (start) and POST (advance).
async fn deck_advance(
    db: &sqlx::PgPool,
    claims: &crate::auth::Claims,
    deck_id: i64,
    review: Option<StudyAdvanceBody>,
) -> Result<Json<StudyAdvance>, (StatusCode, &'static str)> {
    // Verify access via the same visibility check used for viewing decks.
    decks::check_deck_visible(db, deck_id, claims.school_id, claims).await?;

    let deck = sqlx::query!(
        "SELECT id, title FROM decks WHERE id = ? AND school_id = ?",
        deck_id,
        claims.school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Deck not found"))?;

    // Ensure state rows exist for any never-seen cards in the subtree.
    ensure_card_states_for_deck(db, claims.sub, deck_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // If this is an advance, apply the review first.
    let reviewed_card = match review {
        Some(body) => {
            // Validate the card belongs to this deck's subtree before applying.
            let belongs = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM cards c
                WHERE c.id = ?
                  AND c.deck_id IN (
                      WITH RECURSIVE subtree(id) AS (
                          SELECT ?
                          UNION ALL
                          SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
                      )
                      SELECT id FROM subtree
                  )
                "#,
            )
            .bind(body.card_id)
            .bind(deck_id)
            .fetch_one(db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

            if belongs == 0 {
                return Err((StatusCode::BAD_REQUEST, "Card does not belong to this deck"));
            }

            Some(
                crate::handlers::reviews::apply_review(
                    db,
                    claims.sub,
                    body.card_id,
                    body.rating,
                    body.response_time_ms,
                )
                .await?,
            )
        }
        None => None,
    };

    let next_card = next_due_card(db, claims.sub, deck_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Resolve deck options for the predicted-interval computation.
    let options = options_for(db, deck_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let next_card = match next_card {
        Some(row) => Some(
            row_to_study_card(db, row, &options)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?,
        ),
        None => None,
    };

    let counts = deck_counts_for_student(db, claims.sub, deck_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(StudyAdvance {
        next_card,
        reviewed_card,
        counts,
        deck_id: deck.id,
        deck_title: deck.title,
    }))
}
