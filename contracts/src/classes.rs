//! Class-management DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::UserRole;

/// Expected JSON body for creating a class.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateClass {
    pub name: String,
    pub description: Option<String>,
}

/// Expected JSON body for renaming a class.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RenameClass {
    pub name: String,
}

/// Expected JSON body for adding a member to a class.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddMember {
    /// The user to add to the class.
    pub user_id: i64,
}

/// A class as returned to clients.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClassResponse {
    pub id: i64,
    pub school_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
}

/// A single member of a class.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MemberResponse {
    pub user_id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: UserRole,
    pub joined_at: DateTime<Utc>,
}

/// A class together with its full roster.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RosterResponse {
    pub class: ClassResponse,
    pub members: Vec<MemberResponse>,
}
