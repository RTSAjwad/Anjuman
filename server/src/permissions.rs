//! Access authorization (US-4.7 + stage 9).
//!
//! The single source of truth for "what may this user do?". Authorization is
//! **two steps, never conflated**:
//!
//!   1. **Capability (a *role bundle*)** — may this *kind* of user perform this
//!      *kind* of action at all? Pure `Claims → bool`, no resource in scope.
//!   2. **Relationship (user ↔ specific resource)** — *is* this user linked to
//!      *this* resource in the required way (owner / collaborator / class member /
//!      ancestor / descendant / own-card-state)?
//!
//! A decision is `allow iff capability(role) AND relationship(user, resource)`.
//! A role never grants a *particular* resource; it only says *what kind of
//! action* that role may take on resources it is related to in a specific way.
//! The "admin trump" is one relationship rule (admin → may `Manage` any deck in
//! school), not a separate special case.
//!
//! ## Two orthogonal dimensions
//!
//! Deck access has **two independent dimensions** (asking different questions):
//!
//! - **Study** (`DeckAccess::Studyable | ContextOnly | None`) — may the user
//!   *study* the deck (and its subtree)? Grant-based, role-agnostic.
//! - **Governance** (`DeckGovernance::Manage | Contribute | None`) — may the user
//!   *manage* the deck (rename/delete/share) or its *content* (notes/cards)?
//!   Role-gated AND relationship-gated.
//!
//! These are **not** a single ordering: an admin may `Manage` a deck they cannot
//! `Study` (the admin trump grants governance, not study — see US-4.7).
//!
//! ## Layers
//!
//! - **RBAC (thin, coarse):** the `role` claim in the JWT feeds the *role-bundle*
//!   predicates below — the cheap fast-path and coarse gates.
//! - **PBAC seam:** capability-named predicates (`can_manage_decks`, …) are the
//!   single funnel every caller passes through, so migrating role→permission to
//!   a table later is a swap of one function's implementation, not a handler sweep.
//! - **ReBAC (relationship):** `DeckAccess`/`DeckGovernance` + the access/
//!   governance functions answer "what is this user's link to this *specific*
//!   deck".
//!
//! New feature code should call these capabilities rather than inspecting
//! `Claims::role` directly, so permission/scope changes are a single edit here
//! rather than a sweep across handlers.

use sqlx::PgPool;
use axum::http::StatusCode;

use crate::auth::Claims;
use anjuman_contracts::UserRole;

// ---------------------------------------------------------------------------
// Capability predicates (role bundle) — pure `Claims → bool`, no DB.
// ---------------------------------------------------------------------------

/// `can_administer_school` — school-level administration (manage users etc.).
pub fn can_administer_school(claims: &Claims) -> bool {
    claims.role == UserRole::Admin
}

/// `can_manage_users` — may this user manage user accounts?
pub fn can_manage_users(claims: &Claims) -> bool {
    claims.role == UserRole::Admin
}

/// `can_manage_decks` — role bundle for deck management (rename/delete/share/
/// transfer). The relationship term (`deck_governance`) restricts to a deck.
pub fn can_manage_decks(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_manage_notes` — role bundle for note management.
pub fn can_manage_notes(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_manage_presets` — role bundle for deck-options preset management.
pub fn can_manage_presets(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_manage_note_types` — role bundle for note-type management.
pub fn can_manage_note_types(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_manage_tags` — role bundle for tag management.
pub fn can_manage_tags(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_manage_classes` — role bundle for class management.
pub fn can_manage_classes(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_view_dashboard` — role bundle for the teacher/admin dashboard.
pub fn can_view_dashboard(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

/// `can_view_analytics` — role bundle for class analytics.
pub fn can_view_analytics(claims: &Claims) -> bool {
    matches!(claims.role, UserRole::Admin | UserRole::Teacher)
}

// ---------------------------------------------------------------------------
// Role gates (thin helpers preserving the per-resource deny message).
// ---------------------------------------------------------------------------

/// Require the caller is a teacher or admin, returning the resource-specific
/// deny message on failure. The *role-bundle gate* form (no relationship term).
pub fn require_teacher_or_admin(
    claims: &Claims,
    deny_message: &'static str,
) -> Result<(), (StatusCode, &'static str)> {
    if can_manage_decks(claims) {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, deny_message))
    }
}

/// Require the caller is an admin, returning the admin's `school_id` on success
/// (so downstream queries scope by it). School-wide admin gate.
pub fn require_admin(claims: &Claims) -> Result<i64, (StatusCode, &'static str)> {
    if can_administer_school(claims) {
        Ok(claims.school_id)
    } else {
        Err((StatusCode::FORBIDDEN, "Admin role required"))
    }
}

// ---------------------------------------------------------------------------
// Deck study access (ReBAC — grant model).
// ---------------------------------------------------------------------------

/// What study access a user has to a deck (the raw access level, kept compatible
/// with the pre-US-4.7 `DeckAccess` name used across handlers/tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckAccess {
    /// No relationship to the deck or its connected grant set.
    None,
    /// The deck (or an ancestor of it) is granted → the user may study it.
    Studyable,
    /// A *descendant* of the deck is granted → visible as tree context only.
    ContextOnly,
}

/// The study-oriented permission, mirroring `DeckAccess` under a clearer name
/// (stage 9). Kept as a distinct enum so study semantics are never confused with
/// governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeckPerm {
    None,
    VisibleOnly,
    Study,
}

impl From<DeckAccess> for DeckPerm {
    fn from(a: DeckAccess) -> Self {
        match a {
            DeckAccess::None => DeckPerm::None,
            DeckAccess::Studyable => DeckPerm::Study,
            DeckAccess::ContextOnly => DeckPerm::VisibleOnly,
        }
    }
}

/// Compute the user's **study** permission (resource-grant model). Role-agnostic:
/// any authenticated user who holds a grant (owner/collaborator/class, over the
/// subtree) may study — including admins, but an `Admin` role alone is **not** a
/// grant (US-4.7).
pub async fn deck_permission(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
) -> Result<DeckPerm, (StatusCode, &'static str)> {
    Ok(DeckPerm::from(
        deck_access(db, deck_id, claims.school_id, claims).await?,
    ))
}

/// Require the user to hold at least `required` study permission on the deck,
/// returning a 403 otherwise (404 if the deck doesn't exist in the caller's
/// school).
pub async fn require_deck_perm(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
    required: DeckPerm,
) -> Result<(), (StatusCode, &'static str)> {
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
    if granted >= required {
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

// ---------------------------------------------------------------------------
// Deck governance (ReBAC — management, orthogonal to study).
// ---------------------------------------------------------------------------

/// What management access a user has to a deck. Ordered weakest → strongest.
///
/// - `Contribute` — may edit deck *content* (notes/cards): owner **or**
///   collaborator, for teacher/admin only.
/// - `Manage`     — may manage the *deck* (rename/delete/share/transfer): owner,
///   for teacher/admin; **admin trump** for any deck in school.
/// - `None`       — no management access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeckGovernance {
    None,
    Contribute,
    Manage,
}

/// Compute the user's deck **governance** access.
///
/// - `Manage`     if the caller is an admin (trump — any deck in school), or is a
///   teacher/admin owner of the deck.
/// - `Contribute` if the caller is a teacher/admin collaborator (or owner — but
///   an owner already resolves to `Manage`).
/// - `None`       otherwise (including a student owner/collaborator, whose
///   relationship is a *study* grant only — governance is role-gated).
pub async fn deck_governance(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
) -> Result<DeckGovernance, (StatusCode, &'static str)> {
    // Admin trump: admins may manage any deck in their school.
    if claims.role == UserRole::Admin {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM decks WHERE id = $1 AND school_id = $2) AS \"exists!: bool\"",
            deck_id,
            claims.school_id
        )
        .fetch_one(db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
        return Ok(if exists { DeckGovernance::Manage } else { DeckGovernance::None });
    }

    // Governance is role-gated: only teacher/admin reach `Contribute`/`Manage`.
    if !can_manage_decks(claims) {
        return Ok(DeckGovernance::None);
    }

    if is_owner(db, deck_id, claims.school_id, claims.sub).await? {
        return Ok(DeckGovernance::Manage);
    }
    if is_collaborator(db, deck_id, claims.sub).await? {
        return Ok(DeckGovernance::Contribute);
    }

    Ok(DeckGovernance::None)
}

/// Require the user to hold at least `required` governance on the deck (404 if
/// the deck doesn't exist in the caller's school, 403 otherwise).
pub async fn require_deck_governance(
    db: &PgPool,
    claims: &Claims,
    deck_id: i64,
    required: DeckGovernance,
) -> Result<(), (StatusCode, &'static str)> {
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

    let granted = deck_governance(db, claims, deck_id).await?;
    if granted >= required {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "You do not have permission for this action"))
    }
}

// ---------------------------------------------------------------------------
// Relationship primitives (owner / collaborator / grant), non-recursive.
// ---------------------------------------------------------------------------

/// Whether `user_id` is the owner of `deck_id` (in `school_id`).
async fn is_owner(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    user_id: i64,
) -> Result<bool, (StatusCode, &'static str)> {
    let row = sqlx::query_scalar!(
        "SELECT (created_by = $1) AS \"is_owner!: bool\" FROM decks WHERE id = $2 AND school_id = $3",
        user_id,
        deck_id,
        school_id
    )
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    Ok(row.unwrap_or(false))
}

/// Whether `user_id` is a direct collaborator of `deck_id`.
async fn is_collaborator(
    db: &PgPool,
    deck_id: i64,
    user_id: i64,
) -> Result<bool, (StatusCode, &'static str)> {
    let is_collab = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM deck_collaborators WHERE deck_id = $1 AND user_id = $2) AS \"exists!: bool\"",
        deck_id,
        user_id
    )
    .fetch_one(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    Ok(is_collab)
}

/// Whether the user holds a grant on this **specific** deck (owner/collaborator/
/// class-member). Non-recursive — ancestors/descendants are the caller's concern.
///
/// NOTE (US-4.7): there is **no** `Admin → true` shortcut here. Admins hold a
/// grant by the same owner/collaborator/class rules as everyone else; the
/// `Admin` role alone does not grant study access to a deck the admin does not
/// own or collaborate on. (Deck *management* is admin-permissive via
/// `deck_governance`'s admin trump.)
async fn has_grant(
    db: &PgPool,
    deck_id: i64,
    school_id: i64,
    claims: &Claims,
) -> Result<bool, (StatusCode, &'static str)> {
    if is_owner(db, deck_id, school_id, claims.sub).await? {
        return Ok(true);
    }

    if is_collaborator(db, deck_id, claims.sub).await? {
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
