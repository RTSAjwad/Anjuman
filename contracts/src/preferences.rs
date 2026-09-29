//! User preferences — the server-side subset (see `PREFERENCES_SUPPORT.md`).
//!
//! Only preferences that change *shared scheduling behaviour* live here; the
//! client-side / form-factor / Anki-specific preferences are out of scope for
//! the backend.

use serde::{Deserialize, Serialize};

/// The authenticated user's server-side preferences.
///
/// `learn_ahead_minutes` is exposed in minutes (matching Anki's "Learn ahead
/// limit" input, 0–100) — the server stores/reads seconds internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserPreferences {
    /// "Learn ahead limit" — minutes (0–100), default 20.
    pub learn_ahead_minutes: i64,
    /// "Next day starts at" — whole hours past midnight (0–23), default 4.
    pub day_start_hour: i64,
    /// IANA timezone name (e.g. `"Europe/London"`), default `"UTC"`.
    pub timezone: String,
}

/// Request body for `PATCH /preferences`. Fields are optional; omitted fields
/// are left unchanged.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdatePreferences {
    /// "Learn ahead limit" — minutes (0–100).
    pub learn_ahead_minutes: Option<i64>,
    /// "Next day starts at" — whole hours past midnight (0–23).
    pub day_start_hour: Option<i64>,
    /// IANA timezone name (e.g. `"Europe/London"`). Invalid names fall back to `"UTC"`.
    pub timezone: Option<String>,
}
