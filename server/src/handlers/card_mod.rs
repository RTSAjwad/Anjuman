// Card modification endpoints: suspend, unsuspend, bury, unbury, reschedule.
//
// These mirror Anki's card-level controls:
//   - Suspend:     permanently exclude a card from study until unsuspended.
//   - Bury:        hide a card until the next day (auto-reappears tomorrow).
//   - Reschedule:  manually override a card's due date.
//
// All operations are scoped to the authenticated student and their own
// per-card scheduling state (student_card_states).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use chrono::{DateTime, Duration, Utc};

use anjuman_contracts::cards::{
    CardModResponse, MoveCardBody, MoveCardResponse, NoteModResponse, RepositionBody,
    RepositionResponse, RescheduleBody, UnburyQuery,
};

use crate::{auth::AuthUser, db_types::DbCardState, state::AppState};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn ensure_card_state(
    db: &sqlx::PgPool,
    student_id: i64,
    card_id: i64,
) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query!(
        "INSERT INTO student_card_states (student_id, card_id, state, stability, difficulty, reps, lapses) VALUES ($1, $2, 'new', 0.0, 0.0, 0, 0) ON CONFLICT (student_id, card_id) DO NOTHING",
        student_id,
        card_id
    )
    .execute(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(())
}

/// Fetch the current card state for the authenticated student.
async fn fetch_state(
    db: &sqlx::PgPool,
    student_id: i64,
    card_id: i64,
) -> Result<
    (
        String,
        Option<DateTime<Utc>>,
        bool,
        Option<DateTime<Utc>>,
        Option<String>,
    ),
    (StatusCode, &'static str),
> {
    let row = sqlx::query!(
        "SELECT state as \"state!: DbCardState\", due_at, suspended, buried_at, bury_reason FROM student_card_states WHERE student_id = $1 AND card_id = $2",
        student_id,
        card_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Card state not found"))?;

    Ok((
        row.state.as_str().to_string(),
        row.due_at,
        row.suspended,
        row.buried_at,
        row.bury_reason,
    ))
}

/// Fetch the card IDs belonging to a note for the given student.
async fn fetch_note_card_ids(
    db: &sqlx::PgPool,
    student_id: i64,
    note_id: i64,
) -> Result<Vec<i64>, (StatusCode, &'static str)> {
    let rows = sqlx::query!(
        "SELECT scs.card_id FROM student_card_states scs JOIN cards c ON c.id = scs.card_id WHERE scs.student_id = $1 AND c.note_id = $2 ORDER BY scs.card_id",
        student_id,
        note_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(rows.into_iter().map(|r| r.card_id).collect())
}

/// Ensure `student_card_states` rows exist for every card of a note.
async fn ensure_note_card_states(
    db: &sqlx::PgPool,
    student_id: i64,
    note_id: i64,
) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query!(
        r#"
        INSERT INTO student_card_states
            (student_id, card_id, state, stability, difficulty, reps, lapses)
        SELECT $1, c.id, 'new', 0.0, 0.0, 0, 0
        FROM cards c
        WHERE c.note_id = $2
        ON CONFLICT (student_id, card_id) DO NOTHING
        "#,
        student_id,
        note_id
    )
    .execute(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(())
}

/// Read the actual `(suspended, buried_at, bury_reason)` of a note's cards after
/// a bulk operation. All cards are set uniformly, so reading one is sufficient.
async fn fetch_note_result_state(
    db: &sqlx::PgPool,
    student_id: i64,
    note_id: i64,
) -> Result<(bool, Option<DateTime<Utc>>, Option<String>), (StatusCode, &'static str)> {
    let row = sqlx::query!(
        "SELECT scs.suspended, scs.buried_at, scs.bury_reason FROM student_card_states scs JOIN cards c ON c.id = scs.card_id WHERE scs.student_id = $1 AND c.note_id = $2 LIMIT 1",
        student_id,
        note_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "No cards found for this note"))?;

    Ok((row.suspended, row.buried_at, row.bury_reason))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /cards/:card_id/suspend` — Suspend a card (exclude from study).
pub async fn suspend(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    ensure_card_state(&state.db, claims.sub, card_id).await?;

    // Suspending also clears any burial (buried + suspended are mutually
    // exclusive — a card is never both).
    sqlx::query!(
        "UPDATE student_card_states SET suspended = TRUE, buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id = $2",
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `POST /cards/:card_id/unsuspend` — Unsuspend a card.
pub async fn unsuspend(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    ensure_card_state(&state.db, claims.sub, card_id).await?;

    let result = sqlx::query!(
        "UPDATE student_card_states SET suspended = FALSE WHERE student_id = $1 AND card_id = $2",
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card not found"));
    }

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `POST /cards/:card_id/bury` — Bury a card until the next day-start.
pub async fn bury(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    let now = Utc::now();
    ensure_card_state(&state.db, claims.sub, card_id).await?;

    // Store "when buried" and the reason; the "still buried?" check
    // auto-expires the card once `now` passes the next day-start. Suspended
    // cards are not buried (mutually exclusive).
    sqlx::query!(
        "UPDATE student_card_states SET buried_at = $1, bury_reason = 'user_card' WHERE student_id = $2 AND card_id = $3 AND suspended = FALSE",
        now,
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `POST /cards/:card_id/unbury` — Unbury a card.
///
/// By default clears all burial regardless of reason. Pass `?reason=user` or
/// `?reason=sibling` to clear only that group.
pub async fn unbury(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
    Query(params): Query<UnburyQuery>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    ensure_card_state(&state.db, claims.sub, card_id).await?;

    match params.reason.as_deref() {
        Some("user") => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id = $2 AND bury_reason IN ('user_card', 'user_note')",
                claims.sub,
                card_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
        Some("sibling") => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id = $2 AND bury_reason IN ('sibling_new', 'sibling_review', 'sibling_interday')",
                claims.sub,
                card_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
        _ => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id = $2",
                claims.sub,
                card_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
    }

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `PATCH /cards/:card_id/reschedule` — Manually set a card's due date.
///
/// Accepts either an absolute `due_at` or a relative `days` offset from now.
/// If both are provided, `due_at` takes precedence.
pub async fn reschedule(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
    Json(body): Json<RescheduleBody>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    let new_due_at = if let Some(due_at) = body.due_at {
        due_at
    } else if let Some(days) = body.days {
        Utc::now() + Duration::days(days)
    } else {
        return Err((StatusCode::BAD_REQUEST, "Provide either due_at or days"));
    };

    ensure_card_state(&state.db, claims.sub, card_id).await?;

    // Matching Anki: manually rescheduling a card forces it into the review
    // state (queue 2) with the chosen due date. Step index is reset because
    // review cards don't use the learning/relearning step ladder. `state` is
    // the authoritative scheduling signal (not `reps`), so this cleanly moves
    // the card out of new/learning/relearning regardless of its history.
    let result = sqlx::query!(
        "UPDATE student_card_states SET state = 'review', step_index = 0, due_at = $1 WHERE student_id = $2 AND card_id = $3",
        new_due_at,
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card not found"));
    }

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `POST /notes/:note_id/suspend` — Suspend all cards belonging to a note.
///
/// Bulk operation: applies `suspended = 1` to every card of the note for the
/// authenticated student. Affects only the student's own scheduling state.
pub async fn suspend_note(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<Json<NoteModResponse>, (StatusCode, &'static str)> {
    ensure_note_card_states(&state.db, claims.sub, note_id).await?;

    // Suspending also clears burial (mutually exclusive).
    let result = sqlx::query!(
        "UPDATE student_card_states SET suspended = TRUE, buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id IN (SELECT id FROM cards WHERE note_id = $2)",
        claims.sub,
        note_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let affected = result.rows_affected();
    if affected == 0 {
        return Err((StatusCode::NOT_FOUND, "No cards found for this note"));
    }

    let card_ids = fetch_note_card_ids(&state.db, claims.sub, note_id).await?;
    let (suspended, buried_at, bury_reason) =
        fetch_note_result_state(&state.db, claims.sub, note_id).await?;

    Ok(Json(NoteModResponse {
        note_id,
        cards_affected: affected as i64,
        card_ids,
        suspended,
        buried_at,
        bury_reason,
    }))
}

/// `POST /notes/:note_id/unsuspend` — Unsuspend all cards belonging to a note.
pub async fn unsuspend_note(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<Json<NoteModResponse>, (StatusCode, &'static str)> {
    ensure_note_card_states(&state.db, claims.sub, note_id).await?;

    let result = sqlx::query!(
        "UPDATE student_card_states SET suspended = FALSE WHERE student_id = $1 AND card_id IN (SELECT id FROM cards WHERE note_id = $2)",
        claims.sub,
        note_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let affected = result.rows_affected();
    if affected == 0 {
        return Err((StatusCode::NOT_FOUND, "No cards found for this note"));
    }

    let card_ids = fetch_note_card_ids(&state.db, claims.sub, note_id).await?;
    let (suspended, buried_at, bury_reason) =
        fetch_note_result_state(&state.db, claims.sub, note_id).await?;

    Ok(Json(NoteModResponse {
        note_id,
        cards_affected: affected as i64,
        card_ids,
        suspended,
        buried_at,
        bury_reason,
    }))
}

/// `POST /notes/:note_id/bury` — Bury all cards of a note until the next day-start.
pub async fn bury_note(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<Json<NoteModResponse>, (StatusCode, &'static str)> {
    let now = Utc::now();
    ensure_note_card_states(&state.db, claims.sub, note_id).await?;

    // Bury only non-suspended cards (mutually exclusive).
    sqlx::query!(
        "UPDATE student_card_states SET buried_at = $1, bury_reason = 'user_note' WHERE student_id = $2 AND card_id IN (SELECT id FROM cards WHERE note_id = $3) AND suspended = FALSE",
        now,
        claims.sub,
        note_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let card_ids = fetch_note_card_ids(&state.db, claims.sub, note_id).await?;
    let (suspended, buried_at, bury_reason) =
        fetch_note_result_state(&state.db, claims.sub, note_id).await?;

    Ok(Json(NoteModResponse {
        note_id,
        cards_affected: card_ids.len() as i64,
        card_ids,
        suspended,
        buried_at,
        bury_reason,
    }))
}

/// `POST /notes/:note_id/unbury` — Unbury all cards belonging to a note.
///
/// By default clears all burial regardless of reason. Pass `?reason=user` or
/// `?reason=sibling` to clear only that group.
pub async fn unbury_note(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
    Query(params): Query<UnburyQuery>,
) -> Result<Json<NoteModResponse>, (StatusCode, &'static str)> {
    ensure_note_card_states(&state.db, claims.sub, note_id).await?;

    match params.reason.as_deref() {
        Some("user") => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id IN (SELECT id FROM cards WHERE note_id = $2) AND bury_reason IN ('user_card', 'user_note')",
                claims.sub,
                note_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
        Some("sibling") => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id IN (SELECT id FROM cards WHERE note_id = $2) AND bury_reason IN ('sibling_new', 'sibling_review', 'sibling_interday')",
                claims.sub,
                note_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
        _ => {
            sqlx::query!(
                "UPDATE student_card_states SET buried_at = NULL, bury_reason = NULL WHERE student_id = $1 AND card_id IN (SELECT id FROM cards WHERE note_id = $2)",
                claims.sub,
                note_id
            )
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
    }

    let card_ids = fetch_note_card_ids(&state.db, claims.sub, note_id).await?;
    let (suspended, buried_at, bury_reason) =
        fetch_note_result_state(&state.db, claims.sub, note_id).await?;

    Ok(Json(NoteModResponse {
        note_id,
        cards_affected: card_ids.len() as i64,
        card_ids,
        suspended,
        buried_at,
        bury_reason,
    }))
}

/// `PATCH /cards/:card_id/deck` — Move a card to another deck.
///
/// This lets a note's cards be split across decks, matching Anki's model.
/// The target deck must be in the same school and visible to the caller.
pub async fn move_card(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
    Json(body): Json<MoveCardBody>,
) -> Result<Json<MoveCardResponse>, (StatusCode, &'static str)> {
    // Validate the target deck exists in the same school and is accessible.
    let deck = sqlx::query!(
        "SELECT id FROM decks WHERE id = $1 AND school_id = $2",
        body.deck_id,
        claims.school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Deck not found"))?;

    // Authorize: caller must be owner, admin, or collaborator on the target deck.
    crate::handlers::decks::check_deck_collaborator(&state.db, deck.id, claims.school_id, &claims)
        .await?;

    let result = sqlx::query!(
        "UPDATE cards SET deck_id = $1 WHERE id = $2",
        body.deck_id,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card not found"));
    }

    Ok(Json(MoveCardResponse {
        card_id,
        deck_id: body.deck_id,
    }))
}

/// `POST /cards/:card_id/reset` — Reset a card's scheduling back to "new" (Anki's
/// browser "Reset" / "Forget").
///
/// This returns the card to the new queue as if never studied, matching
/// <https://docs.ankiweb.net/faqs/resetting-progress-in-a-deck>. Unlike simple
/// deletion, the row is reset **in place** so the card's Anki-style **flag** and
/// its existence in `student_card_states` are preserved (flags live on the same
/// row — deleting it would silently clear them, which Anki's Reset does not do).
///
/// Scheduling-only fields are zeroed: `state = 'new'`, stability/difficulty/reps/
/// lapses/step_index reset, and `due_at`/`last_reviewed_at`/`buried_at`/
/// `bury_reason`/`leech_at`/`suspended` cleared (a reset card is unsuspended +
/// unburied + not a leech). **Review history (`reviews`) is untouched**, matching
/// Anki (only the separate "Remove card history" add-on deletes it). Note: pre-
/// reset history must be ignored by the future FSRS optimizer (US-2.18b) — Anki
/// models this with a reset marker we don't yet store; see the roadmap caveat.
pub async fn reset(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
) -> Result<Json<CardModResponse>, (StatusCode, &'static str)> {
    // Reset the scheduling state in place, preserving `flag` (and any other
    // non-scheduling column) but clearing every scheduling signal. `state`
    // becomes 'new' — the authoritative "never studied" signal.
    let result = sqlx::query!(
        r#"
        UPDATE student_card_states
        SET state = 'new', stability = 0.0, difficulty = 0.0,
            step_index = 0, reps = 0, lapses = 0,
            due_at = NULL, last_reviewed_at = NULL,
            suspended = FALSE, buried_at = NULL, bury_reason = NULL,
            leech_at = NULL
        WHERE student_id = $1 AND card_id = $2
        "#,
        claims.sub,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card state not found"));
    }

    let (state, due_at, suspended, buried_at, bury_reason) =
        fetch_state(&state.db, claims.sub, card_id).await?;

    Ok(Json(CardModResponse {
        card_id,
        suspended,
        buried_at,
        bury_reason,
        due_at,
        state,
    }))
}

/// `PATCH /cards/:card_id/position` — Reposition a new card in the new-card queue
/// (Anki's "Reposition").
///
/// Writes `cards.position` (the collection-wide new-card queue ordinal, now the
/// authoritative display value per US-4.12a). Reposition is **new-card only**
/// (matching Anki — review/learning/relearning rows have no position).
///
/// Note: Anki's "Shift position of existing cards" renumbers the tail of the new
/// queue so the card can be *inserted* between existing cards. We defer that
/// because our `cards.position` is a global monotonic sequence (not a dense
/// `1..N` per-deck ordinal; see US-4.12a), so "shift existing" would require a
/// dense renumber that is out of scope for this story.
pub async fn reposition(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(card_id): Path<i64>,
    Json(body): Json<RepositionBody>,
) -> Result<Json<RepositionResponse>, (StatusCode, &'static str)> {
    let current = sqlx::query!(
        "SELECT c.id, scs.state::text as state \
         FROM cards c \
         LEFT JOIN student_card_states scs ON scs.card_id = c.id AND scs.student_id = $2 \
         WHERE c.id = $1",
        card_id,
        claims.sub
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Card not found"))?;

    // New-card only: a card with no scheduling state (a NULL-joined row) or an
    // explicit 'new' state may be repositioned. Anything else is rejected.
    let is_new = current
        .state
        .as_deref()
        .is_none_or(|s| s == "new");
    if !is_new {
        return Err((StatusCode::BAD_REQUEST, "Only new cards can be repositioned"));
    }

    let result = sqlx::query!(
        "UPDATE cards SET position = $1 WHERE id = $2",
        body.position,
        card_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Card not found"));
    }

    Ok(Json(RepositionResponse {
        card_id,
        position: body.position,
    }))
}
