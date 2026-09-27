//! Server-local mirror enums for the Postgres `ENUM` column types.
//!
//! sqlx's compile-time `query!`/`query_as!` macros cannot map a custom Postgres
//! `ENUM` directly to a Rust type (see `TypeChecking::return_type_for_id`,
//! which only knows a fixed built-in list). So enum columns are handled with:
//!
//! - **Reads** — an explicit type override in the SQL, e.g.
//!   `role as "role!: DbUserRole"`. sqlx decodes the enum into the `Db*` type
//!   here (thanks to `#[derive(sqlx::Type)]`), keeping the column `NOT NULL`
//!   and avoiding any `.parse()` round-trip.
//! - **Writes** — the enum's lowercase string via `.as_str()`, cast on the SQL
//!   side (`$1::text::user_role`), since the compile-time macros can't accept
//!   a custom enum as a binding directly.
//!
//! These enums deliberately mirror (not reuse) the `anjuman_contracts` enums so
//! that `contracts` stays free of any `sqlx` dependency. Convert to/from the
//! contract types at the handler boundary.

use sqlx::Type;

/// Mirrors the `user_role` enum (`admin`, `teacher`, `student`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum DbUserRole {
    Admin,
    Teacher,
    Student,
}

/// Mirrors the `membership_role` enum (`teacher`, `student`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "membership_role", rename_all = "lowercase")]
pub enum DbMembershipRole {
    Teacher,
    Student,
}

/// Mirrors the `card_state` enum (`new`, `learning`, `review`, `relearning`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "card_state", rename_all = "lowercase")]
pub enum DbCardState {
    New,
    Learning,
    Review,
    Relearning,
}

/// Mirrors the `step_kind` enum (`learning`, `relearning`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "step_kind", rename_all = "lowercase")]
pub enum DbStepKind {
    Learning,
    Relearning,
}

// ---------------------------------------------------------------------------
// Conversions (server-local DB enums <-> contract enum / application strings)
// ---------------------------------------------------------------------------

impl From<anjuman_contracts::UserRole> for DbUserRole {
    fn from(r: anjuman_contracts::UserRole) -> Self {
        match r {
            anjuman_contracts::UserRole::Admin => DbUserRole::Admin,
            anjuman_contracts::UserRole::Teacher => DbUserRole::Teacher,
            anjuman_contracts::UserRole::Student => DbUserRole::Student,
        }
    }
}

impl From<DbUserRole> for anjuman_contracts::UserRole {
    fn from(r: DbUserRole) -> Self {
        match r {
            DbUserRole::Admin => anjuman_contracts::UserRole::Admin,
            DbUserRole::Teacher => anjuman_contracts::UserRole::Teacher,
            DbUserRole::Student => anjuman_contracts::UserRole::Student,
        }
    }
}

impl From<DbMembershipRole> for anjuman_contracts::UserRole {
    fn from(r: DbMembershipRole) -> Self {
        match r {
            DbMembershipRole::Teacher => anjuman_contracts::UserRole::Teacher,
            DbMembershipRole::Student => anjuman_contracts::UserRole::Student,
        }
    }
}

impl From<DbUserRole> for DbMembershipRole {
    fn from(r: DbUserRole) -> Self {
        match r {
            DbUserRole::Teacher => DbMembershipRole::Teacher,
            // Students and admins both fall through to Student; `admin` is
            // rejected by callers before this conversion is used.
            _ => DbMembershipRole::Student,
        }
    }
}

impl DbUserRole {
    /// The role as a lowercase string (e.g. `"teacher"`), used to bind enum
    /// columns via a `::user_role` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbUserRole::Admin => "admin",
            DbUserRole::Teacher => "teacher",
            DbUserRole::Student => "student",
        }
    }
}

impl DbMembershipRole {
    /// The role as a lowercase string (e.g. `"student"`).
    pub fn as_str(self) -> &'static str {
        match self {
            DbMembershipRole::Teacher => "teacher",
            DbMembershipRole::Student => "student",
        }
    }
}

impl DbCardState {
    /// The card state as a lowercase string (e.g. `"learning"`), matching the
    /// application's existing `state: String` wire fields.
    pub fn as_str(self) -> &'static str {
        match self {
            DbCardState::New => "new",
            DbCardState::Learning => "learning",
            DbCardState::Review => "review",
            DbCardState::Relearning => "relearning",
        }
    }
}
