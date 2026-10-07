//! Authentication DTOs.

use serde::{Deserialize, Serialize};

use crate::shared::UserRole;

/// Expected JSON body for the login request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LoginRequest {
    /// The user's email address (used as the login identifier).
    pub email: String,
    /// The user's plain-text password.
    pub password: String,
}

/// JSON response returned on successful login.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LoginResponse {
    /// The JWT access token. Valid for 24 hours.
    pub token: String,
    /// Basic information about the authenticated user.
    pub user: UserResponse,
}

/// Public user info shared across auth surfaces (login, `/me`, registration).
///
/// Consolidated from the previously-duplicated `UserInfo`, `MeResponse`, and
/// `User` structs, which all carried the same fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: UserRole,
    pub school_id: i64,
    /// The user's global capabilities (the role-bundle, resolved server-side).
    /// Resource-scoped permissions (e.g. "may study *this* deck") are surfaced
    /// on the individual resource DTOs (e.g. `DeckResponse.studyable`), not here.
    pub permissions: Permissions,
}

/// The user's global capabilities (the PBAC permission object served by the
/// server). A pure function of the user's role today; later this can be a
/// database-driven mapping without changing the client.
///
/// Only *role-bundle* capabilities live here. Resource-scoped permissions
/// (`can_study_deck`, `deck_governance`) are per-resource and surface on the
/// resource DTOs instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Permissions {
    /// School-level administration (manage users, view all).
    pub can_administer_school: bool,
    /// Manage user accounts.
    pub can_manage_users: bool,
    /// Manage decks (create/rename/delete/share/transfer).
    pub can_manage_decks: bool,
    /// Manage note content.
    pub can_manage_notes: bool,
    /// Manage deck-options presets.
    pub can_manage_presets: bool,
    /// Manage note types and templates.
    pub can_manage_note_types: bool,
    /// Manage the tag namespace.
    pub can_manage_tags: bool,
    /// Manage classes and rosters.
    pub can_manage_classes: bool,
    /// View the teacher/admin dashboard.
    pub can_view_dashboard: bool,
    /// View class analytics.
    pub can_view_analytics: bool,
}
