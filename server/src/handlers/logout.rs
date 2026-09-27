// Logout handler.
//
// Since JWTs are stateless, "logout" means adding the token's unique
// ID (`jti`) to the `revoked_tokens` table. Any subsequent request
// with this token will be rejected even if the token hasn't expired yet.

use axum::{Json, extract::State, http::StatusCode};

use anjuman_contracts::MessageResponse;
use chrono::{TimeZone, Utc};

use crate::{
    auth::{self, AuthUser},
    state::AppState,
};

/// `POST /auth/logout` — Revoke the current JWT.
///
/// The token is extracted from the Authorization header by `AuthUser`,
/// then its `jti` is inserted into `revoked_tokens`. The token is
/// invalidated immediately — it won't work even before its expiry time.
///
/// This endpoint requires authentication (the `AuthUser` extractor).
pub async fn logout(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MessageResponse>, StatusCode> {
    let expires_at = Utc
        .timestamp_opt(claims.exp as i64, 0)
        .single()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    auth::revoke_token(&claims.jti, claims.sub, expires_at, &state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MessageResponse {
        message: "Logged out successfully".to_string(),
    }))
}
