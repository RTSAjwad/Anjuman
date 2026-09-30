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
}
