//! User-management DTOs.

use serde::{Deserialize, Serialize};

use crate::{auth::UserResponse, shared::UserRole};

/// Expected JSON body for creating a new user.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateUser {
    /// The school this user belongs to.
    pub school_id: i64,
    /// Unique email address (doubles as the login identifier).
    pub email: String,
    /// Plain-text password. This is never stored — only the hash is kept.
    pub password: String,
    /// The user's role: `"admin"`, `"teacher"`, or `"student"`.
    pub role: UserRole,
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: String,
}

/// Fields that can be updated on a user. All fields are optional — only the
/// ones provided will be changed.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateUser {
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Full user representation returned to admins.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserDetail {
    pub id: i64,
    pub school_id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: UserRole,
    pub created_at: String,
}

/// Query parameters for `GET /users/search`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SearchQuery {
    pub q: String,
}

/// A single user search result (no `school_id`; scoped to the caller's school).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SearchResult {
    pub id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: UserRole,
}

/// Re-export the canonical user shape for `POST /users` (registration) which
/// returns a full [`UserResponse`].
pub type User = UserResponse;
