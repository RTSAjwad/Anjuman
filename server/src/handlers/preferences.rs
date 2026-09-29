// User preferences — the server-side subset (see `PREFERENCES_SUPPORT.md`).
//
// `user_preferences` is keyed per user. Rows are created lazily: reading a user
// with no row returns the defaults (matching the fallbacks used throughout
// `study.rs`), and writing upserts.
//
// The wire exposes `learn_ahead_minutes` (minutes, 0–100 — matching Anki's
// "Learn ahead limit" input); the DB stores `learn_ahead_seconds`.

use axum::{Json, extract::State, http::StatusCode};

use chrono_tz::Tz;
use std::str::FromStr;

use anjuman_contracts::preferences::{UpdatePreferences, UserPreferences};

use crate::{auth::AuthUser, state::AppState};

// ---------------------------------------------------------------------------
// Defaults (must match the `user_preferences` column defaults)
// ---------------------------------------------------------------------------

const DEFAULT_LEARN_AHEAD_MINUTES: i64 = 20; // 1200 seconds
const DEFAULT_DAY_START_HOUR: i64 = 4;
const DEFAULT_TIMEZONE: &str = "UTC";

/// Resolve a user-supplied timezone to a valid IANA name, falling back to
/// `"UTC"` when the value is empty or unparseable (US-3.1).
fn normalize_timezone(name: &str) -> String {
    if Tz::from_str(name).is_ok() {
        name.to_string()
    } else {
        DEFAULT_TIMEZONE.to_string()
    }
}

/// `GET /preferences` — Return the authenticated user's preferences.
pub async fn get_preferences(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserPreferences>, StatusCode> {
    let row = sqlx::query!(
        "SELECT learn_ahead_seconds, day_start_hour, timezone FROM user_preferences WHERE user_id = $1",
        claims.sub
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(r) => Ok(Json(UserPreferences {
            learn_ahead_minutes: r.learn_ahead_seconds / 60,
            day_start_hour: r.day_start_hour,
            timezone: r.timezone,
        })),
        None => Ok(Json(UserPreferences {
            learn_ahead_minutes: DEFAULT_LEARN_AHEAD_MINUTES,
            day_start_hour: DEFAULT_DAY_START_HOUR,
            timezone: DEFAULT_TIMEZONE.to_string(),
        })),
    }
}

/// `PATCH /preferences` — Update the authenticated user's preferences (upsert).
pub async fn update_preferences(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UpdatePreferences>,
) -> Result<Json<UserPreferences>, StatusCode> {
    // Validate bounds first (matching Anki's in-app ranges).
    if let Some(m) = body.learn_ahead_minutes
        && !(0..=100).contains(&m)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(h) = body.day_start_hour
        && !(0..=23).contains(&h)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Read current (or default) values, then apply the provided fields.
    let current = sqlx::query!(
        "SELECT learn_ahead_seconds, day_start_hour, timezone FROM user_preferences WHERE user_id = $1",
        claims.sub
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let learn_ahead_minutes = body
        .learn_ahead_minutes
        .or_else(|| current.as_ref().map(|r| r.learn_ahead_seconds / 60))
        .unwrap_or(DEFAULT_LEARN_AHEAD_MINUTES);
    let day_start_hour = body
        .day_start_hour
        .or_else(|| current.as_ref().map(|r| r.day_start_hour))
        .unwrap_or(DEFAULT_DAY_START_HOUR);
    let timezone = body
        .timezone
        .as_deref()
        .map(normalize_timezone)
        .or_else(|| current.as_ref().map(|r| r.timezone.clone()))
        .unwrap_or_else(|| DEFAULT_TIMEZONE.to_string());

    sqlx::query!(
        "INSERT INTO user_preferences (user_id, learn_ahead_seconds, day_start_hour, timezone) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id) DO UPDATE SET learn_ahead_seconds = $2, day_start_hour = $3, timezone = $4",
        claims.sub,
        learn_ahead_minutes * 60,
        day_start_hour,
        timezone
    )
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserPreferences {
        learn_ahead_minutes,
        day_start_hour,
        timezone,
    }))
}
