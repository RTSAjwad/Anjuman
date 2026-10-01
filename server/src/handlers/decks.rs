// Deck management.
//
// Teachers (and admins) can create, rename, delete, duplicate, share,
// and publish decks. All operations are scoped to the caller's school.
//
// ## Access model
//
// A user can access a deck if any of these are true:
//   1. They created it (owner).
//   2. They are an admin in the same school.
//   3. The deck is published to the school.
//   4. They are listed as a collaborator on the deck.
//
// ## Role permissions
//
// | Action              | Admin | Teacher | Student |
// |---------------------|-------|---------|---------|
// | Create deck         | ✅    | ✅      | ❌      |
// | Rename deck         | ✅    | ✅ (owner) | ❌  |
// | Delete deck         | ✅    | ✅ (owner) | ❌  |
// | Duplicate deck      | ✅    | ✅      | ❌      |
// | Share deck          | ✅    | ✅ (owner) | ❌  |
// | Publish deck        | ✅    | ✅ (owner) | ❌  |
// | View deck           | ✅    | ✅      | ✅      |

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use chrono::Utc;

use anjuman_contracts::decks::{
    AddDeckToClass, ClassInfo, CollaboratorResponse, CreateDeck, DeckCounts, DeckCountsQuery,
    DeckCountsResponse, DeckDetailResponse, DeckResponse, DeleteDeckQuery, ShareDeck,
    TransferOwner, UpdateDeck,
};
use anjuman_contracts::{MessageResponse, UserRole};

use crate::{
    auth::AuthUser,
    db_types::{DbLimitMode, DbUserRole},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate a per-deck daily-limit input and resolve it to its stored shape.
///
/// Returns `(mode, override_value, today_date)`. A non-`preset` mode requires a
/// non-null override value; `today_only` also records the current date for lazy
/// expiry.
fn resolve_limit_input(
    mode: anjuman_contracts::decks::LimitMode,
    value: Option<i64>,
) -> Result<(DbLimitMode, Option<i64>, Option<chrono::NaiveDate>), (StatusCode, &'static str)> {
    let mode = DbLimitMode::from(mode);
    match mode {
        DbLimitMode::Preset => Ok((mode, value, None)),
        DbLimitMode::ThisDeck | DbLimitMode::TodayOnly => {
            let v = value
                .filter(|v| *v >= 0)
                .ok_or((StatusCode::BAD_REQUEST, "A non-negative limit value is required"))?;
            let today = (mode == DbLimitMode::TodayOnly).then(|| Utc::now().date_naive());
            Ok((mode, Some(v), today))
        }
    }
}

/// Require that the authenticated user is a teacher or admin.
pub fn check_teacher_or_admin(
    claims: &crate::auth::Claims,
) -> Result<(), (StatusCode, &'static str)> {
    match claims.role {
        UserRole::Admin | UserRole::Teacher => Ok(()),
        UserRole::Student => Err((
            StatusCode::FORBIDDEN,
            "Only teachers and admins can manage decks",
        )),
    }
}

/// Check whether the caller can manage (edit/delete/share) a deck.
/// Admins can manage any deck in their school. Teachers must be the owner.
pub async fn check_deck_owner(
    db: &sqlx::PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &crate::auth::Claims,
) -> Result<(), (StatusCode, &'static str)> {
    let row = sqlx::query!(
        "SELECT id, created_by FROM decks WHERE id = $1 AND school_id = $2",
        deck_id,
        school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Deck not found"))?;

    if claims.role == UserRole::Admin {
        return Ok(());
    }

    if row.created_by != claims.sub {
        return Err((
            StatusCode::FORBIDDEN,
            "You can only manage decks you created",
        ));
    }

    Ok(())
}

/// Check whether the caller can manage a deck's content (notes).
/// Admins can manage any deck in their school. Teachers must be
/// the owner OR a collaborator on the deck.
pub async fn check_deck_collaborator(
    db: &sqlx::PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &crate::auth::Claims,
) -> Result<(), (StatusCode, &'static str)> {
    let row = sqlx::query!(
        "SELECT id, created_by FROM decks WHERE id = $1 AND school_id = $2",
        deck_id,
        school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Deck not found"))?;

    if claims.role == UserRole::Admin {
        return Ok(());
    }

    // Owner can manage.
    if row.created_by == claims.sub {
        return Ok(());
    }

    // Check if the user is a collaborator.
    let is_collab = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2) AS "exists!: bool""#,
        deck_id,
        claims.sub
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if is_collab {
        return Ok(());
    }

    Err((
        StatusCode::FORBIDDEN,
        "You can only manage decks you own or collaborate on",
    ))
}

/// Deck access is now centralized in `crate::permissions` (US-4.7). Re-export
/// the names existing call sites use, so `decks::check_deck_visible` etc. keep
/// working while the underlying logic lives in one place.
pub use crate::permissions::{DeckAccess, check_deck_studyable, check_deck_visible, deck_access};

/// Whether the user is an owner, admin, or direct collaborator of the deck —
/// i.e. entitled to see the deck's *administrative* detail (collaborator list
/// and class roster), which is hidden from students who reach the deck only via
/// a subtree/class grant. Shares logic with `get_deck`.
pub async fn is_deck_collaborator(
    db: &sqlx::PgPool,
    deck_id: i64,
    claims: &crate::auth::Claims,
) -> Result<bool, (StatusCode, &'static str)> {
    // Owner or admin.
    let row = sqlx::query!(
        "SELECT created_by FROM decks WHERE id = $1",
        deck_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    if claims.role == UserRole::Admin {
        return Ok(true);
    }
    if let Some(r) = &row {
        if r.created_by == claims.sub {
            return Ok(true);
        }
    }

    // Direct collaborator.
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2",
        deck_id,
        claims.sub
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(count.unwrap_or(0) > 0)
}

/// Fetch a deck row and convert to response DTO.
async fn fetch_deck(
    db: &sqlx::PgPool,
    deck_id: i64,
) -> Result<DeckResponse, (StatusCode, &'static str)> {
    let row = sqlx::query!(
        r#"
        SELECT d.id, d.school_id, d.title, d.description,
               d.created_by, d.parent_id, d.created_at,
               u.email as owner_email,
               u.first_name as owner_first_name,
               u.last_name as owner_last_name,
               d.new_per_day_mode as "new_per_day_mode!: DbLimitMode",
               d.review_per_day_mode as "review_per_day_mode!: DbLimitMode",
               d.new_per_day_override,
               d.review_per_day_override,
               d.new_per_day_today_date,
               d.review_per_day_today_date
        FROM decks d
        JOIN users u ON u.id = d.created_by
        WHERE d.id = $1
        "#,
        deck_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(DeckResponse {
        id: row.id,
        school_id: row.school_id,
        title: row.title,
        description: row.description,
        created_by: row.created_by,
        owner_email: row.owner_email,
        owner_first_name: row.owner_first_name,
        owner_last_name: row.owner_last_name,
        parent_id: row.parent_id,
        created_at: row.created_at,
        new_per_day_mode: Some(row.new_per_day_mode.into()),
        review_per_day_mode: Some(row.review_per_day_mode.into()),
        new_per_day_override: row.new_per_day_override,
        review_per_day_override: row.review_per_day_override,
        new_per_day_today_date: row.new_per_day_today_date,
        review_per_day_today_date: row.review_per_day_today_date,
        new_count: None,
        learning_count: None,
        review_count: None,
        relearning_count: None,
        total_count: None,
        // Default true; overridden where the caller knows the access level
        // (e.g. `get_deck` for context-only student views).
        studyable: true,
    })
}

// ---------------------------------------------------------------------------
// Deck CRUD
// ---------------------------------------------------------------------------

/// `POST /decks` — Create a new deck.
pub async fn create_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateDeck>,
) -> Result<(StatusCode, Json<DeckResponse>), (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;

    // Validate parent exists in same school if provided.
    if let Some(parent_id) = body.parent_id {
        let parent = sqlx::query!(
            "SELECT id FROM decks WHERE id = $1 AND school_id = $2",
            parent_id,
            claims.school_id
        )
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        if parent.is_none() {
            return Err((StatusCode::BAD_REQUEST, "Parent deck not found"));
        }
    }

    // Validate per-deck daily-limit override inputs.
    let (new_mode, new_override, new_today) =
        resolve_limit_input(body.new_per_day_mode, body.new_per_day_override)?;
    let (review_mode, review_override, review_today) =
        resolve_limit_input(body.review_per_day_mode, body.review_per_day_override)?;

    let result = sqlx::query!(
        r#"
        INSERT INTO decks (school_id, title, description, parent_id, created_by, created_at,
            new_per_day_mode, review_per_day_mode, new_per_day_override, review_per_day_override,
            new_per_day_today_date, review_per_day_today_date)
        VALUES ($1, $2, $3, $4, $5, $6, $7::text::limit_mode, $8::text::limit_mode, $9, $10, $11, $12)
        RETURNING id
        "#,
        claims.school_id,
        body.title,
        body.description,
        body.parent_id,
        claims.sub,
        Utc::now(),
        new_mode.as_str(),
        review_mode.as_str(),
        new_override,
        review_override,
        new_today,
        review_today,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let deck = fetch_deck(&state.db, result.id).await?;
    Ok((StatusCode::CREATED, Json(deck)))
}

/// `PATCH /decks/:id/rename` — Rename or move a deck.
///
/// If `parent_id` is provided, the deck is moved to a new parent.
/// Set `parent_id: null` to make it a root deck.
/// Cycle detection prevents a deck from becoming its own ancestor.
pub async fn rename_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Json(body): Json<UpdateDeck>,
) -> Result<Json<DeckResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_collaborator(&state.db, deck_id, claims.school_id, &claims).await?;

    if let Some(title) = &body.title {
        if title.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Title cannot be empty"));
        }
        sqlx::query!("UPDATE decks SET title = $1 WHERE id = $2", title, deck_id)
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(parent_id) = body.parent_id {
        // If moving to a parent, validate it exists and check for cycles.
        if let Some(pid) = parent_id {
            let parent = sqlx::query!(
                "SELECT id FROM decks WHERE id = $1 AND school_id = $2",
                pid,
                claims.school_id
            )
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or((StatusCode::BAD_REQUEST, "Parent deck not found"))?;

            // Cycle detection: walk up from the proposed parent to check
            // we don't encounter `deck_id` (which would mean deck_id is an
            // ancestor of the proposed parent — a cycle).
            let mut current = parent.id;
            loop {
                if current == deck_id {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Cannot move a deck under one of its own descendants",
                    ));
                }
                let next = sqlx::query_scalar!("SELECT parent_id FROM decks WHERE id = $1", current)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
                match next {
                    Some(Some(n)) => current = n,
                    _ => break,
                }
            }
        }

        sqlx::query!(
            "UPDATE decks SET parent_id = $1 WHERE id = $2",
            parent_id,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(description) = body.description {
        sqlx::query!(
            "UPDATE decks SET description = $1 WHERE id = $2",
            description,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(options_id) = body.options_id {
        // Validate the preset exists in the same school if one is provided.
        if let Some(oid) = options_id {
            let opt = crate::deck_options::get_options(&state.db, oid)
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "Deck options not found"))?;
            if opt.school_id != claims.school_id {
                return Err((StatusCode::BAD_REQUEST, "Deck options not found"));
            }
        }
        sqlx::query!(
            "UPDATE decks SET options_id = $1 WHERE id = $2",
            options_id,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    // Per-deck daily-limit overflow (`new_per_day_mode`, `review_per_day_mode`,
    // and their override values). Re-validate against the *current* stored
    // override value when only the mode changes.
    if let Some(mode) = body.new_per_day_mode {
        let current_override = sqlx::query_scalar!(
            "SELECT new_per_day_override FROM decks WHERE id = $1",
            deck_id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        let (m, v, today) = resolve_limit_input(mode, current_override.or(body.new_per_day_override.flatten()))?;
        sqlx::query!(
            "UPDATE decks SET new_per_day_mode = $1::text::limit_mode, new_per_day_override = $2, new_per_day_today_date = $3 WHERE id = $4",
            m.as_str(), v, today, deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    } else if body.new_per_day_override.is_some() {
        let v = body.new_per_day_override.unwrap();
        sqlx::query!(
            "UPDATE decks SET new_per_day_override = $1 WHERE id = $2",
            v,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(mode) = body.review_per_day_mode {
        let current_override = sqlx::query_scalar!(
            "SELECT review_per_day_override FROM decks WHERE id = $1",
            deck_id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        let (m, v, today) = resolve_limit_input(mode, current_override.or(body.review_per_day_override.flatten()))?;
        sqlx::query!(
            "UPDATE decks SET review_per_day_mode = $1::text::limit_mode, review_per_day_override = $2, review_per_day_today_date = $3 WHERE id = $4",
            m.as_str(), v, today, deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    } else if body.review_per_day_override.is_some() {
        let v = body.review_per_day_override.unwrap();
        sqlx::query!(
            "UPDATE decks SET review_per_day_override = $1 WHERE id = $2",
            v,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    let deck = fetch_deck(&state.db, deck_id).await?;
    Ok(Json(deck))
}

/// `DELETE /decks/:id` — Delete a deck permanently.
///
/// Query parameter `cascade=true` also deletes all child decks recursively.
/// Without it, children become root decks (their parent_id is set to NULL).
pub async fn delete_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Query(params): Query<DeleteDeckQuery>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    if params.cascade.unwrap_or(false) {
        // Delete the deck and all descendant decks via recursive CTE.
        sqlx::query!(
            r#"
            DELETE FROM decks
            WHERE id IN (
                WITH RECURSIVE subtree(id) AS (
                    SELECT $1::BIGINT
                    UNION ALL
                    SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
                )
                SELECT id FROM subtree
            )
            "#,
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        Ok(Json(MessageResponse {
            message: "Deck and all subdecks deleted".to_string(),
        }))
    } else {
        // Unparent children first, then delete the deck.
        sqlx::query!(
            "UPDATE decks SET parent_id = NULL WHERE parent_id = $1",
            deck_id
        )
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        sqlx::query!("DELETE FROM decks WHERE id = $1", deck_id)
            .execute(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        Ok(Json(MessageResponse {
            message: "Deck deleted".to_string(),
        }))
    }
}

/// `POST /decks/:id/duplicate` — Create a copy of an existing deck.
///
/// Copies the deck shell, all notes, and all cards. The new deck belongs
/// to the caller and records the source via `original_deck_id`.
pub async fn duplicate_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
) -> Result<(StatusCode, Json<DeckResponse>), (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;

    // Fetch the source deck — must be visible to the caller.
    let source = sqlx::query!(
        r#"
        SELECT id, title, description, parent_id, created_by
        FROM decks
        WHERE id = $1 AND school_id = $2
        "#,
        deck_id,
        claims.school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Deck not found"))?;

    // Visibility check: must be owner, admin, or collaborator.
    let can_access = claims.role == UserRole::Admin || source.created_by == claims.sub;

    if !can_access {
        let is_collab = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2) AS "exists!: bool""#,
            deck_id,
            claims.sub
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        if !is_collab {
            return Err((StatusCode::FORBIDDEN, "You do not have access to this deck"));
        }
    }

    let new_title = format!("{} (copy)", source.title);

    // Wrap the entire duplication in a transaction — if anything fails,
    // the partial copy is rolled back.
    let mut tx = crate::db::begin_immediate(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let result = sqlx::query!(
        r#"
        INSERT INTO decks (school_id, title, description, parent_id, created_by, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
        claims.school_id,
        new_title,
        source.description,
        source.parent_id,
        claims.sub,
        Utc::now()
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let new_deck_id = result.id;

    // Copy all notes from the source deck into the new deck.
    // Notes are deck-independent, so we find notes via their cards' deck_id.
    let notes = sqlx::query!(
        "SELECT DISTINCT n.id, n.note_type_id, n.fields_json FROM notes n JOIN cards c ON c.note_id = n.id WHERE c.deck_id = $1 ORDER BY n.id",
        deck_id
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    for note in &notes {
        let note_result = sqlx::query!(
            "INSERT INTO notes (note_type_id, fields_json, created_at) VALUES ($1, $2, $3) RETURNING id",
            note.note_type_id,
            note.fields_json as _,
            Utc::now()
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let new_note_id = note_result.id;

        let cards = sqlx::query!(
            "SELECT template_id FROM cards WHERE note_id = $1 AND deck_id = $2",
            note.id,
            deck_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        for card in &cards {
            sqlx::query!(
                "INSERT INTO cards (note_id, deck_id, template_id, created_at) VALUES ($1, $2, $3, $4)",
                new_note_id,
                new_deck_id,
                card.template_id,
                Utc::now()
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        }
    }

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let deck = fetch_deck(&state.db, new_deck_id).await?;
    Ok((StatusCode::CREATED, Json(deck)))
}

// ---------------------------------------------------------------------------
// Sharing & publishing
// ---------------------------------------------------------------------------

/// `POST /decks/:id/share` — Share a deck with another teacher.
///
/// The target user must be a teacher or admin in the same school.
/// If the deck is already shared with them, the request is idempotent.
pub async fn share_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Json(body): Json<ShareDeck>,
) -> Result<Json<CollaboratorResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    // Verify the target user exists in the same school and is a teacher/admin.
    let target = sqlx::query!(
        "SELECT id, email, first_name, last_name, role as \"role!: DbUserRole\" FROM users WHERE id = $1 AND school_id = $2",
        body.user_id,
        claims.school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "User not found in your school"))?;

    if target.role == DbUserRole::Student {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot share a deck with a student",
        ));
    }

    if target.role == DbUserRole::Admin {
        return Err((
            StatusCode::BAD_REQUEST,
            "Admins already have access to all decks",
        ));
    }

    // Prevent sharing with the deck owner (they already have full access).
    let deck = sqlx::query!("SELECT created_by FROM decks WHERE id = $1", deck_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if body.user_id == deck.created_by {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot share a deck with its owner",
        ));
    }

    let now = Utc::now();

    // ON CONFLICT DO NOTHING makes this idempotent.
    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, $3) ON CONFLICT (deck_id, user_id) DO NOTHING",
        deck_id,
        body.user_id,
        now
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(CollaboratorResponse {
        user_id: body.user_id,
        email: target.email,
        first_name: target.first_name,
        last_name: target.last_name,
        shared_at: now,
    }))
}

/// `DELETE /decks/:id/share/:user_id` — Remove a collaborator from a deck.
pub async fn unshare_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((deck_id, user_id)): Path<(i64, i64)>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    let result = sqlx::query!(
        "DELETE FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2",
        deck_id,
        user_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Collaborator not found"));
    }

    Ok(Json(MessageResponse {
        message: "Collaborator removed".to_string(),
    }))
}

/// `PATCH /decks/:id/owner` — Transfer deck ownership to another teacher.
///
/// The current owner (or an admin) can transfer ownership. The new owner
/// must be a teacher in the same school. The old owner is automatically
/// added as a collaborator so they don't lose access.
pub async fn transfer_owner(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Json(body): Json<TransferOwner>,
) -> Result<Json<DeckResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    // Fetch current owner for collaborator insertion.
    let current = sqlx::query!("SELECT created_by FROM decks WHERE id = $1", deck_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Idempotent: transferring to the current owner does nothing.
    if body.user_id == current.created_by {
        let deck = fetch_deck(&state.db, deck_id).await?;
        return Ok(Json(deck));
    }

    // Verify target is a teacher in the same school.
    let target = sqlx::query!(
        "SELECT id, role as \"role!: DbUserRole\" FROM users WHERE id = $1 AND school_id = $2",
        body.user_id,
        claims.school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "User not found in your school"))?;

    if target.role != DbUserRole::Teacher {
        return Err((
            StatusCode::BAD_REQUEST,
            "Ownership can only be transferred to a teacher",
        ));
    }

    let now = Utc::now();

    // Update the owner and add the old owner as a collaborator atomically.
    let mut tx = crate::db::begin_immediate(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    sqlx::query!(
        "UPDATE decks SET created_by = $1 WHERE id = $2",
        body.user_id,
        deck_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    sqlx::query!(
        "INSERT INTO deck_collaborators (deck_id, user_id, shared_at) VALUES ($1, $2, $3) ON CONFLICT (deck_id, user_id) DO NOTHING",
        deck_id,
        current.created_by,
        now
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let deck = fetch_deck(&state.db, deck_id).await?;
    Ok(Json(deck))
}

/// Add a deck to a class so students in that class can study it.
///
/// Only the deck owner, an admin, or a teacher member of the class can do this.
pub async fn add_deck_to_class(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
    Json(body): Json<AddDeckToClass>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    // Verify the class belongs to the same school.
    let _class = sqlx::query!(
        "SELECT id, name FROM classes WHERE id = $1 AND school_id = $2",
        body.class_id,
        claims.school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "Class not found"))?;

    let now = Utc::now();

    sqlx::query!(
        "INSERT INTO deck_classes (deck_id, class_id, added_at) VALUES ($1, $2, $3) ON CONFLICT (deck_id, class_id) DO NOTHING",
        deck_id,
        body.class_id,
        now
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(MessageResponse {
        message: "Deck added to class".to_string(),
    }))
}

/// Remove a deck from a class.
pub async fn remove_deck_from_class(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((deck_id, class_id)): Path<(i64, i64)>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    check_teacher_or_admin(&claims)?;
    check_deck_owner(&state.db, deck_id, claims.school_id, &claims).await?;

    let result = sqlx::query!(
        "DELETE FROM deck_classes WHERE deck_id = $1 AND class_id = $2",
        deck_id,
        class_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Deck is not assigned to this class"));
    }

    Ok(Json(MessageResponse {
        message: "Deck removed from class".to_string(),
    }))
}

/// List classes a deck is assigned to.
pub async fn list_deck_classes(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
) -> Result<Json<Vec<ClassInfo>>, (StatusCode, &'static str)> {
    check_deck_visible(&state.db, deck_id, claims.school_id, &claims).await?;

    // The class roster is administrative meta — hidden from a student who isn't
    // the owner/admin/direct collaborator of this deck (e.g. a context-only
    // ancestor under US-2.19).
    let classes = if is_deck_collaborator(&state.db, deck_id, &claims).await? {
        let rows = sqlx::query!(
            r#"
            SELECT c.id, c.name
            FROM deck_classes dc
            JOIN classes c ON c.id = dc.class_id
            WHERE dc.deck_id = $1
            ORDER BY c.name
            "#,
            deck_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        rows.into_iter()
            .map(|r| ClassInfo {
                id: r.id,
                name: r.name,
            })
            .collect()
    } else {
        vec![]
    };

    Ok(Json(classes))
}

// ---------------------------------------------------------------------------
// Viewing
// ---------------------------------------------------------------------------

/// `GET /decks/:id` — Get a deck with its collaborators list.
///
/// The caller must have access: owner, admin, published deck, or collaborator.
pub async fn get_deck(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(deck_id): Path<i64>,
) -> Result<Json<DeckDetailResponse>, (StatusCode, &'static str)> {
    check_deck_visible(&state.db, deck_id, claims.school_id, &claims).await?;

    let mut deck = fetch_deck(&state.db, deck_id).await?;

    // A student viewing a context-only ancestor gets `studyable = false`
    // (US-2.19); everyone else stays studyable.
    deck.studyable = deck_access(&state.db, deck_id, claims.school_id, &claims).await?
        == DeckAccess::Studyable;

    // Owner, admins, and collaborators can see who else is on the deck.
    let is_collab = is_deck_collaborator(&state.db, deck_id, &claims).await?;

    let collaborators = if is_collab {
        let rows = sqlx::query!(
            r#"
            SELECT dc.user_id, u.email, u.first_name, u.last_name, dc.shared_at
            FROM deck_collaborators dc
            JOIN users u ON u.id = dc.user_id
            WHERE dc.deck_id = $1
            ORDER BY u.email
            "#,
            deck_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        rows.into_iter()
            .map(|r| CollaboratorResponse {
                user_id: r.user_id,
                email: r.email,
                first_name: r.first_name,
                last_name: r.last_name,
                shared_at: r.shared_at,
            })
            .collect()
    } else {
        vec![]
    };

    // Fetch classes the deck is assigned to. Gated the same way as the
    // collaborator list: a student who isn't the owner/admin/collaborator of
    // *this* deck (e.g. a context-only ancestor under US-2.19) must not see its
    // class roster.
    let deck_classes: Vec<ClassInfo> = if is_collab {
        let class_rows = sqlx::query!(
            r#"
            SELECT c.id, c.name
            FROM deck_classes dc
            JOIN classes c ON c.id = dc.class_id
            WHERE dc.deck_id = $1
            ORDER BY c.name
            "#,
            deck_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        class_rows
            .into_iter()
            .map(|r| ClassInfo {
                id: r.id,
                name: r.name,
            })
            .collect()
    } else {
        vec![]
    };

    Ok(Json(DeckDetailResponse {
        deck,
        collaborators,
        classes: deck_classes,
    }))
}

/// `GET /decks` — List decks visible to the caller.
///
/// - Teachers see their own decks + shared + published.
/// - Admins see all decks in their school.
/// - Students see published decks + decks assigned to their classes.
pub async fn list_decks(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DeckResponse>>, (StatusCode, &'static str)> {
    if claims.role == UserRole::Admin {
        let rows = sqlx::query!(
            r#"
            SELECT d.id, d.school_id, d.title, d.description,
                   d.created_by, d.parent_id, d.created_at,
                   u.email as owner_email,
                   u.first_name as owner_first_name,
                   u.last_name as owner_last_name,
                   (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) as "card_count!: i64"
            FROM decks d
            JOIN users u ON u.id = d.created_by
            WHERE d.school_id = $1
            ORDER BY d.created_at DESC
            "#,
            claims.school_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let decks: Vec<DeckResponse> = rows
            .into_iter()
            .map(|r| DeckResponse {
                id: r.id,
                school_id: r.school_id,
                title: r.title,
                description: r.description,
                created_by: r.created_by,
                owner_email: r.owner_email,
                owner_first_name: r.owner_first_name,
                owner_last_name: r.owner_last_name,
                parent_id: r.parent_id,
                created_at: r.created_at,
                new_per_day_mode: None,
                review_per_day_mode: None,
                new_per_day_override: None,
                review_per_day_override: None,
                new_per_day_today_date: None,
                review_per_day_today_date: None,
                new_count: None,
                learning_count: None,
                review_count: None,
                relearning_count: None,
                total_count: Some(r.card_count),
                studyable: true,
            })
            .collect();
        return Ok(Json(decks));
    }

    if claims.role == UserRole::Teacher {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT d.id, d.school_id, d.title, d.description,
                   d.created_by, d.parent_id, d.created_at,
                   u.email as owner_email,
                   u.first_name as owner_first_name,
                   u.last_name as owner_last_name,
                   (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) as "card_count!: i64"
            FROM decks d
            JOIN users u ON u.id = d.created_by
            LEFT JOIN deck_collaborators dc ON dc.deck_id = d.id
            WHERE d.school_id = $1
              AND (d.created_by = $2 OR dc.user_id = $3)
            ORDER BY d.created_at DESC
            "#,
            claims.school_id,
            claims.sub,
            claims.sub
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let decks: Vec<DeckResponse> = rows
            .into_iter()
            .map(|r| DeckResponse {
                id: r.id,
                school_id: r.school_id,
                title: r.title,
                description: r.description,
                created_by: r.created_by,
                owner_email: r.owner_email,
                owner_first_name: r.owner_first_name,
                owner_last_name: r.owner_last_name,
                parent_id: r.parent_id,
                created_at: r.created_at,
                new_per_day_mode: None,
                review_per_day_mode: None,
                new_per_day_override: None,
                review_per_day_override: None,
                new_per_day_today_date: None,
                review_per_day_today_date: None,
                new_count: None,
                learning_count: None,
                review_count: None,
                relearning_count: None,
                total_count: Some(r.card_count),
                studyable: true,
            })
            .collect();
        return Ok(Json(decks));
    }

    // Students: the decks they're granted, expanded to the connected subtree
    // (ancestors as read-only context + descendants as inherited grants), with
    // per-deck `studyable` and study counts (US-2.19).

    // Phase 1 — the decks this student is *directly* granted, via class
    // membership or collaboration.
    let granted_ids = sqlx::query!(
        r#"
        SELECT DISTINCT d.id
        FROM decks d
        LEFT JOIN deck_classes dcl ON dcl.deck_id = d.id
        LEFT JOIN class_members cm ON cm.class_id = dcl.class_id AND cm.user_id = $1
        LEFT JOIN deck_collaborators dc ON dc.deck_id = d.id AND dc.user_id = $1
        WHERE d.school_id = $2
          AND (cm.user_id IS NOT NULL OR dc.user_id IS NOT NULL)
        "#,
        claims.sub,
        claims.school_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .into_iter()
    .map(|r| r.id)
    .collect::<Vec<i64>>();

    // Phase 2 — expand to ancestors + descendants of the granted set (connected
    // subtree), marking each deck `studyable` iff it *or an ancestor* is granted.
    // Descendants of a grant are studyable; ancestors of a grant are context-only.
    let visible = sqlx::query!(
        r#"
        WITH RECURSIVE
        granted(id) AS (SELECT UNNEST($1::BIGINT[])),
        -- Descendants of each grant (inherited study access).
        descendants(id) AS (
            SELECT id FROM granted
            UNION
            SELECT d.id FROM decks d JOIN descendants s ON d.parent_id = s.id
        ),
        -- Ancestors of each grant (read-only context).
        ancestors(id) AS (
            SELECT id FROM granted
            UNION
            SELECT d.parent_id FROM decks d JOIN ancestors s ON d.id = s.id
                WHERE d.parent_id IS NOT NULL
        ),
        expanded AS (SELECT id FROM descendants UNION SELECT id FROM ancestors)
        SELECT d.id, d.school_id, d.title, d.description, d.created_by, d.parent_id, d.created_at,
               u.email as owner_email, u.first_name as owner_first_name, u.last_name as owner_last_name,
               (d.id IN (SELECT id FROM descendants)) AS "studyable!: bool"
        FROM decks d
        JOIN users u ON u.id = d.created_by
        WHERE d.id IN (SELECT id FROM expanded)
        ORDER BY d.created_at DESC
        "#,
        &granted_ids
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let mut decks: Vec<DeckResponse> = Vec::new();
    for r in visible {
        let deck_id = r.id;
        let studyable = r.studyable;

        let counts =
            crate::handlers::study::deck_counts_for_student(&state.db, claims.sub, deck_id)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let total = sqlx::query!(
            r#"
            SELECT COUNT(*) as "total!: i64"
            FROM cards c
            WHERE c.deck_id IN (
                WITH RECURSIVE subtree(id) AS (
                    SELECT $1::BIGINT
                    UNION ALL
                    SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
                )
                SELECT id FROM subtree
            )
            "#,
            deck_id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        decks.push(DeckResponse {
            id: deck_id,
            school_id: r.school_id,
            title: r.title,
            description: r.description,
            created_by: r.created_by,
            owner_email: r.owner_email,
            owner_first_name: r.owner_first_name,
            owner_last_name: r.owner_last_name,
            parent_id: r.parent_id,
            created_at: r.created_at,
            new_per_day_mode: None,
            review_per_day_mode: None,
            new_per_day_override: None,
            review_per_day_override: None,
            new_per_day_today_date: None,
            review_per_day_today_date: None,
            new_count: Some(counts.new_count),
            learning_count: Some(counts.learning_count),
            review_count: Some(counts.review_count),
            relearning_count: Some(counts.relearning_count),
            total_count: Some(total.total),
            studyable,
        });
    }

    Ok(Json(decks))
}

// ---------------------------------------------------------------------------
// Deck counts
// ---------------------------------------------------------------------------

/// `GET /decks/counts` — Lightweight per-deck card counts.
///
/// For students, returns per-state counts (their own scheduling state).
/// For teachers/admins, returns only `total_count` (per-state counts are
/// per-student and deferred).
pub async fn deck_counts(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(params): Query<DeckCountsQuery>,
) -> Result<Json<DeckCountsResponse>, (StatusCode, &'static str)> {
    let mut result = Vec::new();

    if claims.role == UserRole::Student {
        // Students: decks they're granted (class membership or collaboration),
        // expanded to their descendants (subtree grant, US-2.19). Counts are
        // returned for studyable decks only, not read-only ancestor context.
        let rows = sqlx::query!(
            r#"
            WITH RECURSIVE
            granted(id) AS (
                SELECT d.id
                FROM decks d
                LEFT JOIN deck_classes dcl ON dcl.deck_id = d.id
                LEFT JOIN class_members cm ON cm.class_id = dcl.class_id AND cm.user_id = $1
                LEFT JOIN deck_collaborators dc ON dc.deck_id = d.id AND dc.user_id = $1
                WHERE d.school_id = $2 AND (cm.user_id IS NOT NULL OR dc.user_id IS NOT NULL)
            ),
            subtree(id) AS (
                SELECT id FROM granted
                UNION
                SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
            )
            SELECT DISTINCT id FROM subtree
            "#,
            claims.sub,
            claims.school_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        for row in rows {
            let Some(deck_id) = row.id else { continue };
            if let Some(filter) = params.deck_id {
                if deck_id != filter {
                    continue;
                }
            }

            let counts =
                crate::handlers::study::deck_counts_for_student(&state.db, claims.sub, deck_id)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

            // `total_count` is the physical card count in the subtree, not
            // subject to daily limits. It isn't tracked by the study helper,
            // so compute it here for this response.
            let total = sqlx::query!(
                r#"
                SELECT COUNT(*) as "total!: i64"
                FROM cards c
                WHERE c.deck_id IN (
                    WITH RECURSIVE subtree(id) AS (
                        SELECT $1::BIGINT
                        UNION ALL
                        SELECT d.id FROM decks d JOIN subtree s ON d.parent_id = s.id
                    )
                    SELECT id FROM subtree
                )
                "#,
                deck_id
            )
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

            result.push(DeckCounts {
                deck_id,
                new_count: counts.new_count,
                learning_count: counts.learning_count,
                review_count: counts.review_count,
                relearning_count: counts.relearning_count,
                total_count: total.total,
            });
        }
    } else {
        // Teacher/admin: total count only.
        let rows = sqlx::query!(
            r#"
            SELECT d.id, (SELECT COUNT(*) FROM cards c WHERE c.deck_id IN (
                WITH RECURSIVE subtree(id) AS (
                    SELECT d.id
                    UNION ALL
                    SELECT x.id FROM decks x JOIN subtree s ON x.parent_id = s.id
                )
                SELECT id FROM subtree
            )) as "total!: i64"
            FROM decks d
            WHERE d.school_id = $1
            "#,
            claims.school_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        for row in rows {
            let deck_id = row.id;
            if let Some(filter) = params.deck_id {
                if deck_id != filter {
                    continue;
                }
            }
            result.push(DeckCounts {
                deck_id,
                new_count: 0,
                learning_count: 0,
                review_count: 0,
                relearning_count: 0,
                total_count: row.total,
            });
        }
    }

    Ok(Json(DeckCountsResponse { decks: result }))
}
