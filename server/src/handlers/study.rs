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

use chrono::{DateTime, Duration, Timelike, Utc};

use anjuman_contracts::study::{StudyAdvance, StudyAdvanceBody, StudyCard, StudyCounts};

use crate::{
    auth::AuthUser,
    db_types::DbCardState,
    handlers::{decks, reviews::hard_step_delay},
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
#[derive(sqlx::FromRow)]
struct CardRow {
    id: i64,
    note_id: i64,
    template_id: i64,
    note_type_id: i64,
    fields_json: sqlx::types::Json<serde_json::Value>,
    state: DbCardState,
    due_at: Option<DateTime<Utc>>,
    stability: f64,
    difficulty: f64,
    reps: i64,
    lapses: i64,
    flag: i64,
    suspended: bool,
    buried_at: Option<DateTime<Utc>>,
    bury_reason: Option<String>,
    step_index: i64,
}

/// Compute the start of the current "study day" as a `DateTime<Utc>`, given a
/// day-start hour (0-23). The day is anchored to UTC for now (no timezone).
///
/// Preserves the original integer `% 86400` day-boundary logic by operating on
/// whole elapsed seconds since midnight, then re-wrapping in a `DateTime<Utc>`.
fn day_start_utc(now: DateTime<Utc>, day_start_hour: i64) -> DateTime<Utc> {
    let secs_of_day = now.num_seconds_from_midnight() as i64;
    let start_seconds = day_start_hour * 3600;
    let offset = if secs_of_day >= start_seconds {
        secs_of_day - start_seconds
    } else {
        secs_of_day + 86400 - start_seconds
    };
    now - Duration::seconds(offset)
}

/// Resolve the student's start-of-day hour (Anki's "Next day starts at",
/// default 4 AM). Falls back to 4 when no preference row exists.
async fn day_start_hour(db: &sqlx::PgPool, student_id: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT day_start_hour FROM user_preferences WHERE user_id = $1",
        student_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(4)
}

/// Start of the current study day for a student.
async fn start_of_day(db: &sqlx::PgPool, student_id: i64) -> DateTime<Utc> {
    let hour = day_start_hour(db, student_id).await;
    day_start_utc(Utc::now(), hour)
}

/// Resolve the student's learn-ahead limit (in seconds).
///
/// This is a per-user option (Anki's "Learn ahead limit", default 20 minutes).
/// Falls back to 1200 when no preference row exists.
async fn learn_ahead_seconds(db: &sqlx::PgPool, student_id: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT learn_ahead_seconds FROM user_preferences WHERE user_id = $1",
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
        INSERT INTO student_card_states
            (student_id, card_id, state, stability, difficulty, reps, lapses)
        SELECT $1, c.id, 'new', 0.0, 0.0, 0, 0
        FROM cards c
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT $2::bigint
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
        ON CONFLICT (student_id, card_id) DO NOTHING
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
        WHERE student_id = $1 AND reviewed_at >= $2
        "#,
        student_id,
        day_start
    )
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((row.new_seen, row.review_seen))
}

/// Today's already-reviewed counts, bucketed by the card's own deck within a
/// deck subtree. Used for per-subdeck daily-limit caps (US-2.7).
///
/// Returns one row per deck in `deck_id`'s subtree that has seen activity,
/// as `(deck_id, new_seen, review_seen)`.
struct SubdeckSeen {
    deck_id: i64,
    new_seen: i64,
    review_seen: i64,
}

async fn subdeck_seen_today(
    db: &sqlx::PgPool,
    student_id: i64,
    deck_id: i64,
) -> Result<Vec<SubdeckSeen>, StatusCode> {
    let day_start = start_of_day(db, student_id).await;
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE subtree(id) AS (
            SELECT $1::bigint
            UNION ALL
            SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
        )
        SELECT c.deck_id,
               COALESCE(SUM(CASE WHEN r.state_before = 'new' THEN 1 ELSE 0 END), 0) as "new_seen!: i64",
               COALESCE(SUM(CASE WHEN r.state_before IN ('review', 'relearning') THEN 1 ELSE 0 END), 0) as "review_seen!: i64"
        FROM reviews r
        JOIN cards c ON c.id = r.card_id
        WHERE r.student_id = $2 AND r.reviewed_at >= $3
          AND c.deck_id IN (SELECT id FROM subtree)
        GROUP BY c.deck_id
        "#,
        deck_id,
        student_id,
        day_start
    )
    .fetch_all(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(rows
        .into_iter()
        .map(|r| SubdeckSeen {
            deck_id: r.deck_id,
            new_seen: r.new_seen,
            review_seen: r.review_seen,
        })
        .collect())
}

/// Per-deck remaining daily budget for every deck in `deck_id`'s subtree,
/// as a map of `deck_id -> (new_remaining, review_remaining)`. Each remaining
/// = effective_limit - seen_today (floored at 0).
async fn subdeck_remaining_budgets(
    db: &sqlx::PgPool,
    student_id: i64,
    deck_id: i64,
    today: chrono::NaiveDate,
) -> Result<std::collections::HashMap<i64, (i64, i64)>, StatusCode> {
    // Discover the subtree deck ids.
    let ids: Vec<i64> = sqlx::query!(
        r#"
        WITH RECURSIVE subtree(id) AS (
            SELECT $1::bigint
            UNION ALL
            SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
        )
        SELECT id as "id!: i64" FROM subtree ORDER BY id
        "#,
        deck_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| r.id)
    .collect();

    let seen = subdeck_seen_today(db, student_id, deck_id).await?;

    let mut budgets = std::collections::HashMap::with_capacity(ids.len());
    for id in ids {
        let (new_limit, review_limit) =
            crate::deck_options::effective_daily_limits(db, id, today)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let s = seen.iter().find(|s| s.deck_id == id);
        let new_seen = s.map(|s| s.new_seen).unwrap_or(0);
        let review_seen = s.map(|s| s.review_seen).unwrap_or(0);
        budgets.insert(id, ((new_limit - new_seen).max(0), (review_limit - review_seen).max(0)));
    }

    Ok(budgets)
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
    let (new_seen, review_seen) = seen_today(db, student_id).await?;
    let learn_ahead = learn_ahead_seconds(db, student_id).await;
    let day_start = start_of_day(db, student_id).await;
    let now = Utc::now();
    let learn_ahead_deadline = now + Duration::seconds(learn_ahead);

    // Selected deck's effective limits + per-subdeck remaining budgets (US-2.7).
    let (selected_new, selected_review) =
        crate::deck_options::effective_daily_limits(db, deck_id, day_start.date_naive())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let selected_new_remaining = (selected_new - new_seen).max(0);
    let selected_review_remaining = (selected_review - review_seen).max(0);

    let budgets = subdeck_remaining_budgets(db, student_id, deck_id, day_start.date_naive())
        .await?;

    // Per-subdeck physical due-now counts, grouped by the card's own deck, so
    // each subdeck's own remaining budget can cap its contribution.
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE subtree(id) AS (
            SELECT $1::bigint
            UNION ALL
            SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
        )
        SELECT c.deck_id as "deck_id!: i64",
            COALESCE(SUM(CASE WHEN (scs.state = 'new') AND (scs.suspended = FALSE AND (scs.buried_at IS NULL OR scs.buried_at < $2)) THEN 1 ELSE 0 END), 0) as "new_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'learning' AND scs.due_at <= $3 AND (scs.suspended = FALSE AND (scs.buried_at IS NULL OR scs.buried_at < $2)) THEN 1 ELSE 0 END), 0) as "learning_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'review' AND scs.due_at <= $4 AND (scs.suspended = FALSE AND (scs.buried_at IS NULL OR scs.buried_at < $2)) THEN 1 ELSE 0 END), 0) as "review_total!: i64",
            COALESCE(SUM(CASE WHEN scs.state = 'relearning' AND scs.due_at <= $3 AND (scs.suspended = FALSE AND (scs.buried_at IS NULL OR scs.buried_at < $2)) THEN 1 ELSE 0 END), 0) as "relearning_total!: i64"
        FROM cards c
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = $5
        WHERE c.deck_id IN (SELECT id FROM subtree)
        GROUP BY c.deck_id
        "#,
        deck_id,
        day_start,
        learn_ahead_deadline,
        now,
        student_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut new_total = 0i64;
    let mut learning_total = 0i64;
    let mut review_total = 0i64;
    let mut relearning_total = 0i64;

    for r in rows {
        let (new_cap, review_cap) = budgets.get(&r.deck_id).copied().unwrap_or((0, 0));
        new_total += r.new_total.min(new_cap);
        learning_total += r.learning_total;
        review_total += r.review_total.min(review_cap);
        relearning_total += r.relearning_total;
    }

    Ok(StudyCounts {
        new_count: new_total.min(selected_new_remaining),
        learning_count: learning_total,
        review_count: review_total.min(selected_review_remaining),
        relearning_count: relearning_total,
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
    let (new_seen, review_seen) = seen_today(db, student_id).await?;
    let learn_ahead = learn_ahead_seconds(db, student_id).await;
    let day_start = start_of_day(db, student_id).await;
    let now = Utc::now();

    // Selected deck's effective daily limits + its own remaining budget (US-2.6)
    // and the per-subdeck remaining budgets (US-2.7).
    let (selected_new, selected_review) =
        crate::deck_options::effective_daily_limits(db, deck_id, day_start.date_naive())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let selected_new_remaining = (selected_new - new_seen).max(0);
    let selected_review_remaining = (selected_review - review_seen).max(0);

    // Per-subdeck budgets as a stable, id-ordered set of parallel arrays for the
    // UNNEST join below.
    let budgets = subdeck_remaining_budgets(db, student_id, deck_id, day_start.date_naive()).await?;
    let mut subdeck_ids: Vec<i64> = budgets.keys().copied().collect();
    subdeck_ids.sort_unstable();
    let subdeck_new_rem: Vec<i64> = subdeck_ids.iter().map(|id| budgets[id].0).collect();
    let subdeck_review_rem: Vec<i64> = subdeck_ids.iter().map(|id| budgets[id].1).collect();

    // The selected deck's new-card gather order (from its effective preset;
    // display order is always taken from the selected deck, not subdecks).
    let options = options_for(db, deck_id).await?;
    let gather = options.new_gather_order;
    // Deterministic per-student-day seed for the random orders (divergence from
    // Anki's per-session seed; see DECK_OPTIONS_SUPPORT.md).
    let seed = format!("{student_id}:{}", day_start.date_naive());

    // The ORDER BY ordering key for *new* cards. Review/learning cards are
    // unaffected here (their class sorts first, and due_at breaks ties).
    let new_order = match gather {
        anjuman_contracts::deck_options::NewGatherOrder::Deck => {
            "cd.title ASC, c.position ASC".to_string()
        }
        anjuman_contracts::deck_options::NewGatherOrder::DeckThenRandomNotes => {
            format!("cd.title ASC, md5('{seed}:' || c.note_id::text) ASC")
        }
        anjuman_contracts::deck_options::NewGatherOrder::Ascending => {
            "c.position ASC".to_string()
        }
        anjuman_contracts::deck_options::NewGatherOrder::Descending => {
            "c.position DESC".to_string()
        }
        anjuman_contracts::deck_options::NewGatherOrder::RandomNotes => {
            format!("md5('{seed}:' || c.note_id::text) ASC")
        }
        anjuman_contracts::deck_options::NewGatherOrder::RandomCards => {
            format!("md5('{seed}:' || c.id::text) ASC")
        }
    };

    // The new-card *sort* order (US-2.9), applied after gathering. This is the
    // final display order of the gathered new cards.
    let sort = options.new_sort_order;
    let new_sort = match sort {
        anjuman_contracts::deck_options::NewSortOrder::Gathered => String::new(),
        anjuman_contracts::deck_options::NewSortOrder::CardTypeThenGathered => {
            "tpl.ord ASC, c.position ASC".to_string()
        }
        anjuman_contracts::deck_options::NewSortOrder::CardTypeThenRandom => {
            format!("tpl.ord ASC, md5('{seed}:' || c.id::text) ASC")
        }
        anjuman_contracts::deck_options::NewSortOrder::RandomNoteThenCardType => {
            format!("md5('{seed}:' || c.note_id::text) ASC, tpl.ord ASC")
        }
        anjuman_contracts::deck_options::NewSortOrder::Random => {
            format!("md5('{seed}:' || c.id::text) ASC")
        }
    };
    let sort_comma = if new_sort.is_empty() { "" } else { ", " };

    // Class ordering (US-2.10 + US-2.11): assign each state an explicit rank.
    // Resolve the relative order of {interday learning, review, new} in Rust and
    // emit a fixed CASE. Intraday learning is always first (rank 0).
    //
    // Pairwise constraints:
    //  - new vs review (new_review_order): before → new < review; else review < new.
    //  - interday vs review (interday_order): before → interday < review;
    //    after → interday > review; mix → interday < review (gathered-first).
    //  - interday vs new is unspecified → interday < new (gathering order).
    let interday_after_review = options.interday_order == anjuman_contracts::deck_options::InterdayOrder::After;
    let new_before_review = options.new_review_order == anjuman_contracts::deck_options::NewReviewOrder::Before;

    // Start from the default order [interday, review, new] and apply the two
    // flips pairwise; then assign ranks 1..3.
    let rank_interday: i64;
    let rank_review: i64;
    let rank_new: i64;

    // Review's rank vs new.
    rank_review = if new_before_review { 3 } else { 2 };
    rank_new = if new_before_review { 2 } else { 3 };
    // Interday's rank: before review/new by default (rank 1); if after, place it
    // after review (and new) at rank 4.
    rank_interday = if interday_after_review { 4 } else { 1 };

    let class_case = format!(
        "CASE \
            WHEN scs.state IN ('learning','relearning') AND COALESCE(dos.seconds,0) >= 86400 THEN {rank_interday} \
            WHEN scs.state IN ('learning','relearning') THEN 0 \
            WHEN scs.state = 'review' THEN {rank_review} \
            ELSE {rank_new} END"
    );

    let sql = format!(
        r#"
        SELECT c.id, c.note_id, c.template_id, n.note_type_id, n.fields_json,
               scs.state, scs.due_at, scs.stability, scs.difficulty, scs.reps, scs.lapses,
               scs.flag, scs.suspended, scs.buried_at, scs.bury_reason, scs.step_index
        FROM cards c
        JOIN notes n ON n.id = c.note_id
        JOIN decks cd ON cd.id = c.deck_id
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = $1
        LEFT JOIN deck_option_steps dos
            ON dos.options_id = COALESCE(cd.options_id, 0)
           AND dos.kind = (CASE scs.state WHEN 'relearning' THEN 'relearning' ELSE 'learning' END)::step_kind
           AND dos.step_index = scs.step_index
        JOIN note_type_templates tpl ON tpl.id = c.template_id
        JOIN UNNEST($7::bigint[], $8::bigint[], $9::bigint[]) AS budget(deck_id, new_rem, review_rem)
            ON budget.deck_id = c.deck_id
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT $2::bigint
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
          AND scs.suspended = FALSE
          AND (scs.buried_at IS NULL OR scs.buried_at < $3)
          AND (
                (scs.state IN ('learning', 'relearning') AND scs.due_at <= $4)
             OR (scs.state = 'review' AND scs.due_at <= $4 AND $5::bigint > 0 AND budget.review_rem > 0)
             OR (scs.state = 'new' AND $6::bigint > 0 AND budget.new_rem > 0)
          )
        ORDER BY
            {class_case},
            scs.due_at ASC NULLS LAST,
            CASE WHEN scs.state = 'new' THEN 0 ELSE 1 END,
            {new_order}{sort_comma}{new_sort},
            c.id ASC
        LIMIT 1
        "#
    );

    let row = sqlx::query_as::<_, CardRow>(&sql)
        .bind(student_id)
        .bind(deck_id)
        .bind(day_start)
        .bind(now)
        .bind(selected_review_remaining)
        .bind(selected_new_remaining)
        .bind(&subdeck_ids)
        .bind(&subdeck_new_rem)
        .bind(&subdeck_review_rem)
        .fetch_optional(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if row.is_some() {
        return Ok(row);
    }

    // Learn-ahead fallback: no card is actually due, but a learning/relearning
    // card is due within the learn-ahead window. Show the soonest one.
    let learn_ahead_deadline = now + Duration::seconds(learn_ahead);
    let ahead = sqlx::query_as!(
        CardRow,
        r#"
        SELECT c.id, c.note_id, c.template_id, n.note_type_id, n.fields_json,
               scs.state as "state!: DbCardState", scs.due_at, scs.stability as "stability: f64",
               scs.difficulty as "difficulty: f64", scs.reps, scs.lapses,
               scs.flag as "flag: i64", scs.suspended as "suspended: bool",
               scs.buried_at, scs.bury_reason, scs.step_index as "step_index: i64"
        FROM cards c
        JOIN notes n ON n.id = c.note_id
        JOIN student_card_states scs
            ON scs.card_id = c.id AND scs.student_id = $1
        WHERE c.deck_id IN (
            WITH RECURSIVE subtree(id) AS (
                SELECT $2::bigint
                UNION ALL
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT id FROM subtree
        )
          AND scs.suspended = FALSE
          AND (scs.buried_at IS NULL OR scs.buried_at < $3)
          AND scs.state IN ('learning', 'relearning')
          AND scs.due_at > $4
          AND scs.due_at <= $5
        ORDER BY scs.due_at ASC
        LIMIT 1
        "#,
        student_id,
        deck_id,
        day_start,
        now,
        learn_ahead_deadline
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

    let fields: serde_json::Map<String, serde_json::Value> = c
        .fields_json
        .0
        .as_object()
        .map(|m| m.clone())
        .unwrap_or_default();

    let rendered = note_types::render_card(&nt.templates, c.template_id, &fields)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Predicted intervals (in seconds) for each rating, mirroring the exact
    // scheduling rules in `apply_review` so the pre-submission hints are
    // truthful.
    let predicted_interval = predict_intervals(&c, options);

    Ok(StudyCard {
        card_id: c.id,
        note_id: c.note_id,
        front: rendered.front,
        back: rendered.back,
        state: c.state.as_str().to_string(),
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
    let now = Utc::now();

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
            .map(|due| (now - due).num_days().max(0) as u32)
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

    match c.state {
        DbCardState::New => {
            // Again: first learning step; Hard: average of first two steps;
            // Good: next step; Easy: graduate.
            map.insert(
                "1".to_string(),
                learning_steps.first().copied().unwrap_or(60),
            );
            map.insert(
                "2".to_string(),
                hard_step_delay(learning_steps, 0),
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
        DbCardState::Learning => {
            // Again: first step; Hard: repeats current step; Good: next step (or
            // graduate); Easy: graduate.
            map.insert(
                "1".to_string(),
                learning_steps.first().copied().unwrap_or(60),
            );
            map.insert("2".to_string(), hard_step_delay(learning_steps, c.step_index));
            let next_idx = (c.step_index as usize + 1).min(learning_steps.len());
            let good = if next_idx >= learning_steps.len() {
                fsrs_intervals
                    .as_ref()
                    .map(|f| interval_secs(f[2]))
                    .unwrap_or(86400)
            } else {
                learning_steps.get(next_idx).copied().unwrap_or(600)
            };
            map.insert("3".to_string(), good);
            let easy = fsrs_intervals
                .as_ref()
                .map(|f| interval_secs(f[3]))
                .unwrap_or(86400 * 4);
            map.insert("4".to_string(), easy);
        }
        DbCardState::Review => {
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
        DbCardState::Relearning => {
            // Again: first relearning step; Hard: repeats current step; Good: next
            // step (or graduate).
            map.insert(
                "1".to_string(),
                relearning_steps.first().copied().unwrap_or(600),
            );
            map.insert(
                "2".to_string(),
                hard_step_delay(relearning_steps, c.step_index),
            );
            let next_idx = (c.step_index as usize + 1).min(relearning_steps.len());
            let good = if next_idx >= relearning_steps.len() {
                fsrs_intervals
                    .as_ref()
                    .map(|f| interval_secs(f[2]))
                    .unwrap_or(86400)
            } else {
                relearning_steps.get(next_idx).copied().unwrap_or(600)
            };
            map.insert("3".to_string(), good);
            let easy = fsrs_intervals
                .as_ref()
                .map(|f| interval_secs(f[3]))
                .unwrap_or(86400 * 4);
            map.insert("4".to_string(), easy);
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
        "SELECT id, title FROM decks WHERE id = $1 AND school_id = $2",
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
                WHERE c.id = $1
                  AND c.deck_id IN (
                      WITH RECURSIVE subtree(id) AS (
                          SELECT $2::bigint
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
