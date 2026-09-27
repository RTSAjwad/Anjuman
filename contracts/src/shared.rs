//! Types shared across multiple API areas.

use serde::{Deserialize, Serialize};

/// A user's role in the system.
///
/// Mirrors the `role` CHECK constraint on the `users` table
/// (`admin` / `teacher` / `student`). The lowercase JSON representation comes
/// from `#[serde(rename_all = "lowercase")]`, and `Display` produces the same
/// strings for database storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum UserRole {
    Admin,
    Teacher,
    Student,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Teacher => write!(f, "teacher"),
            UserRole::Student => write!(f, "student"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(UserRole::Admin),
            "teacher" => Ok(UserRole::Teacher),
            "student" => Ok(UserRole::Student),
            _ => Err(()),
        }
    }
}

/// A generic message-only response.
///
/// Consolidates the many ad-hoc `{ "message": "…" }` response shapes (history:
/// `MessageResponse` in several handlers, `DeletedResponse`, `LogoutResponse`)
/// into a single shared type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MessageResponse {
    pub message: String,
}
