// User search handler.
//
//   GET /users/search?q=...
//
// Allows admins and teachers to search for users in their school
// by first name, last name, or email. Students cannot search.
// Admins are excluded from results (they can't be added to classes
// or shared with on decks).

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use anjuman_contracts::users::{SearchQuery, SearchResult};
use anjuman_contracts::UserRole;

use crate::{auth::AuthUser, state::AppState};

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// `GET /users/search?q=john` — Search for users by name or email.
///
/// Scoped to the caller's school. Returns up to 20 results.
/// Only teachers and admins can search. Admins are excluded from results.
pub async fn search_users(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, &'static str)> {
    // Only teachers and admins can search for users.
    if claims.role == UserRole::Student {
        return Err((StatusCode::FORBIDDEN, "Students cannot search for users"));
    }

    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }

    let pattern = format!("%{}%", query);

    let rows = sqlx::query!(
        r#"
        SELECT id, email, first_name, last_name, role
        FROM users
        WHERE school_id = ?
          AND role != 'admin'
          AND (
            first_name LIKE ? COLLATE NOCASE
            OR last_name LIKE ? COLLATE NOCASE
            OR email LIKE ? COLLATE NOCASE
          )
        ORDER BY
          CASE
            WHEN first_name LIKE ? COLLATE NOCASE THEN 0
            WHEN last_name LIKE ? COLLATE NOCASE THEN 1
            ELSE 2
          END,
          first_name
        LIMIT 20
        "#,
        claims.school_id,
        pattern,
        pattern,
        pattern,
        pattern,
        pattern
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|r| {
            Ok::<_, (StatusCode, &'static str)>(SearchResult {
                id: r.id.expect("user.id is NOT NULL in schema"),
                email: r.email,
                first_name: r.first_name,
                last_name: r.last_name,
                role: r
                    .role
                    .parse()
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid role"))?,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(Json(results))
}
