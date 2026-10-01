//! Deck-access authorization (US-4.7).
//!
//! A single source of truth for "what may this user do to this deck?". Deck
//! permissions are **resource-grants**, not role-gates: whether a user may study
//! (or manage) a deck is a function of their *relationship* to it — owner,
//! collaborator, or class membership, extended over the subtree per US-2.19 —
//! not of their role label alone. Roles only shape which grants a user tends to
//! hold; they are not a blanket shortcut.
//!
//! New deck feature code should call [`deck_permission`] / [`require_deck_perm`]
//! rather than inspecting `Claims::role` directly, so permission/scope changes
//! are a single edit here rather than a sweep across handlers.

use sqlx::PgPool;
use axum::http::StatusCode;

use crate::auth::Claims;

/// What a user may do to a deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckPerm {
    /// The deck (or an ancestor of it) is granted → the user may study it.
    Study,
    /// The deck is visible as tree context only (a *descendant* is granted, but
    /// not the deck itself or an ancestor). Not studyable.
    VisibleOnly,
    /// No relationship to the deck or its connected grant set.
    None,
}

/// The raw access level, kept compatible with the pre-US-4.7 `DeckAccess` name
/// used across handlers/tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckAccess {
    /// No relationship to the deck or its connected grant set.
    None,
    /// The deck (or an ancestor of it) is granted → the user may study it.
    Studyable,
    /// A *descendant* of the deck is granted → visible as tree context only.
    ContextOnly,
}

/// Compute the user's deck permission (resource-grant model).
pub async fn deck_permission(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
) -> Result<DeckPerm, (StatusCode, &'static str)> {
    match deck_access(db, deck_id, claims.school_id, claims).await? {
        DeckAccess::Studyable => Ok(DeckPerm::Study),
        DeckAccess::ContextOnly => Ok(DeckPerm::VisibleOnly),
        DeckAccess::None => Ok(DeckPerm::None),
    }
}

/// Require the user to hold at least `required` on the deck, returning a 403
/// otherwise (404 if the deck doesn't exist in the caller's school).
pub async fn require_deck_perm(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
    required: DeckPerm,
) -> Result<(), (StatusCode, &'static str)> {
    // 404 for a deck that doesn't exist in this school.
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM decks WHERE id = $1 AND school_id = $2) AS \"exists!: bool\"",
        deck_id,
        claims.school_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    if !exists {
        return Err((StatusCode::NOT_FOUND, "Deck not found"));
    }

    let granted = deck_permission(db, claims, deck_id).await?;
    // `Study` is the strongest grant; `VisibleOnly` is weaker, `None` weakest.
    let ok = match required {
        DeckPerm::Study => granted == DeckPerm::Study,
        DeckPerm::VisibleOnly => matches!(granted, DeckPerm::Study | DeckPerm::VisibleOnly),
        DeckPerm::None => true,
    };
    if ok {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "You do not have permission for this action"))
    }
}

/// `check_deck_visible` — a deck is visible if it is studyable (self or ancestor
/// grant) *or* is a context-only ancestor of a granted descendant.
pub async fn check_deck_visible(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &Claims,
) -> Result<(), (StatusCode, &'static str)> {
    // 404 for a deck that doesn't exist in this school.
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM decks WHERE id = $1 AND school_id = $2) AS \"exists!: bool\"",
        deck_id,
        school_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    if !exists {
        return Err((StatusCode::NOT_FOUND, "Deck not found"));
    }

    match deck_access(db, deck_id, school_id, claims).await? {
        DeckAccess::None => Err((StatusCode::FORBIDDEN, "You do not have access to this deck")),
        DeckAccess::Studyable | DeckAccess::ContextOnly => Ok(()),
    }
}

/// `check_deck_studyable` — a deck must be *studyable* (self or ancestor grant),
/// not merely visible as context.
pub async fn check_deck_studyable(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &Claims,
) -> Result<(), (StatusCode, &'static str)> {
    check_deck_visible(db, deck_id, school_id, claims).await?;
    if deck_access(db, deck_id, school_id, claims).await? == DeckAccess::ContextOnly {
        return Err((StatusCode::FORBIDDEN, "This deck is shared as context only"));
    }
    Ok(())
}

/// Whether the user holds a grant on this **specific** deck (owner/collaborator/
/// class-member). Non-recursive — ancestors/descendants are the caller's concern.
///
/// NOTE (US-4.7): there is **no** `Admin → true` shortcut here. Admins hold a
/// grant by the same owner/collaborator/class rules as everyone else; the
/// `Admin` role alone does not grant study access to a deck the admin does not
/// own or collaborate on. (Deck *management* remains admin-permissive — see
/// `decks::check_deck_owner` / `check_deck_collaborator`.)
async fn has_grant(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &Claims,
) -> Result<bool, (StatusCode, &'static str)> {
    let row = sqlx::query!(
        "SELECT created_by FROM decks WHERE id = $1 AND school_id = $2",
        deck_id,
        school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    let Some(row) = row else {
        return Ok(false);
    };
    if row.created_by == claims.sub {
        return Ok(true);
    }

    // Collaborator?
    let is_collab = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2) AS \"exists!: bool\"",
        deck_id,
        claims.sub
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    if is_collab {
        return Ok(true);
    }

    // In a class the deck is assigned to?
    let in_class = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM deck_classes dcl
            JOIN class_members cm ON cm.class_id = dcl.class_id
            WHERE dcl.deck_id = $1 AND cm.user_id = $2
        ) AS \"exists!: bool\"",
        deck_id,
        claims.sub
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(in_class)
}

/// Compute the user's access to `deck_id` under the subtree access model.
///
/// - `Studyable` if the deck itself **or any ancestor** holds a grant;
/// - `ContextOnly` if not studyable but some **descendant** holds a grant;
/// - `None` otherwise.
pub async fn deck_access(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &Claims,
) -> Result<DeckAccess, (StatusCode, &'static str)> {
    // Ancestors (inclusive) of deck_id.
    let ancestor_ids = sqlx::query!(
        r#"
        WITH RECURSIVE ancestors(id) AS (
            SELECT $1::BIGINT
            UNION ALL
            SELECT d.parent_id FROM decks d JOIN ancestors a ON d.id = a.id
        )
        SELECT id FROM ancestors WHERE id IS NOT NULL
        "#,
        deck_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    for a in ancestor_ids.into_iter().filter_map(|r| r.id) {
        if has_grant(db, a, school_id, claims).await? {
            return Ok(DeckAccess::Studyable);
        }
    }

    // Descendants of deck_id (exclusive), to detect a context-only ancestor.
    let descendant_ids = sqlx::query!(
        r#"
        WITH RECURSIVE descendants(id) AS (
            SELECT d.id FROM decks d WHERE d.parent_id = $1
            UNION ALL
            SELECT d.id FROM decks d JOIN descendants s ON d.parent_id = s.id
        )
        SELECT id FROM descendants
        "#,
        deck_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    for d in descendant_ids.into_iter().filter_map(|r| r.id) {
        if has_grant(db, d, school_id, claims).await? {
            return Ok(DeckAccess::ContextOnly);
        }
    }

    Ok(DeckAccess::None)
}
