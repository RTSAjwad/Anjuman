// Admin-only user management.
//
// School administrators can view, update, and delete any user within
// their school. Teachers and students are denied access to these endpoints.
//
// Every handler here:
//   1. Requires authentication (via `AuthUser`).
//   2. Checks that the caller has the `Admin` role.
//   3. Scopes all database operations to the admin's `school_id` so an
//      admin at School A cannot touch users at School B.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use anjuman_contracts::users::{UpdateUser, UserDetail};
use anjuman_contracts::{MessageResponse, UserRole};

use crate::{
    auth::AuthUser,
    db_types::DbUserRole,
    handlers::users::hash_password,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reject the request if the authenticated user is not an admin.
///
/// Returns the admin's `school_id` so downstream queries can scope by it.
/// This is the single gate that enforces admin-only access across all
/// the handlers in this module.
fn check_admin(claims: &crate::auth::Claims) -> Result<i64, (StatusCode, &'static str)> {
    if claims.role != UserRole::Admin {
        return Err((StatusCode::FORBIDDEN, "Admin role required"));
    }
    Ok(claims.school_id)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /users/:id` — Get a single user by id.
///
/// The admin can only view users within their own school.
pub async fn get_user(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<UserDetail>, (StatusCode, &'static str)> {
    let school_id = check_admin(&claims)?;

    let row = sqlx::query!(
        r#"
        SELECT id, school_id, email, first_name, last_name, role as "role!: DbUserRole", created_at
        FROM users
        WHERE id = $1 AND school_id = $2
        "#,
        user_id,
        school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "User not found"))?;

    Ok(Json(UserDetail {
        id: row.id,
        school_id: row.school_id,
        email: row.email,
        first_name: row.first_name,
        last_name: row.last_name,
        role: row.role.into(),
        created_at: row.created_at,
    }))
}

/// `PATCH /users/:id` — Update a user's email, password, and/or role.
///
/// Only the fields present in the JSON body are changed. If `password`
/// is included, it is hashed with Argon2 before storage.
/// The admin can only update users within their own school.
pub async fn update_user(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Json(body): Json<UpdateUser>,
) -> Result<Json<UserDetail>, (StatusCode, &'static str)> {
    let school_id = check_admin(&claims)?;

    // Verify the target user exists and belongs to the admin's school.
    let _ = sqlx::query!(
        "SELECT id FROM users WHERE id = $1 AND school_id = $2",
        user_id,
        school_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
    .ok_or((StatusCode::NOT_FOUND, "User not found"))?;

    // Hash the new password (if any) before opening the transaction.
    let new_hash = match &body.password {
        Some(password) => Some(
            hash_password(password)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password hashing failed"))?,
        ),
        None => None,
    };

    // Apply all field updates in a single transaction so the whole PATCH is
    // atomic.
    let mut tx = crate::db::begin_immediate(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if let Some(email) = &body.email {
        sqlx::query!("UPDATE users SET email = $1 WHERE id = $2", email, user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    (
                        StatusCode::CONFLICT,
                        "A user with that email already exists",
                    )
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
                }
            })?;
    }

    if let Some(hash) = &new_hash {
        sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            hash,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(role) = &body.role {
        sqlx::query!("UPDATE users SET role = $1::text::user_role WHERE id = $2", DbUserRole::from(*role).as_str(), user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(first_name) = &body.first_name {
        sqlx::query!(
            "UPDATE users SET first_name = $1 WHERE id = $2",
            first_name,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    if let Some(last_name) = &body.last_name {
        sqlx::query!(
            "UPDATE users SET last_name = $1 WHERE id = $2",
            last_name,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
    }

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    // Fetch and return the updated user.
    let row = sqlx::query!(
        r#"
        SELECT id, school_id, email, first_name, last_name, role as "role!: DbUserRole", created_at
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    Ok(Json(UserDetail {
        id: row.id,
        school_id: row.school_id,
        email: row.email,
        first_name: row.first_name,
        last_name: row.last_name,
        role: row.role.into(),
        created_at: row.created_at,
    }))
}

/// `DELETE /users/:id` — Delete a user.
///
/// The admin can only delete users within their own school.
/// Deleting a user cascades to their class memberships, card states,
/// reviews via the database's
/// `ON DELETE CASCADE` foreign keys.
pub async fn delete_user(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<MessageResponse>, (StatusCode, &'static str)> {
    let school_id = check_admin(&claims)?;

    let result = sqlx::query!(
        "DELETE FROM users WHERE id = $1 AND school_id = $2",
        user_id,
        school_id
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "User not found"));
    }

    Ok(Json(MessageResponse {
        message: "User deleted".to_string(),
    }))
}

/// `GET /users` — List all users in the admin's school.
///
/// Returns users ordered by creation date (newest first).
pub async fn list_users(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserDetail>>, (StatusCode, &'static str)> {
    let school_id = check_admin(&claims)?;

    let rows = sqlx::query!(
        r#"
        SELECT id, school_id, email, first_name, last_name, role as "role!: DbUserRole", created_at
        FROM users
        WHERE school_id = $1
        ORDER BY created_at DESC
        "#,
        school_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let users: Vec<UserDetail> = rows
        .into_iter()
        .map(|r| UserDetail {
            id: r.id,
            school_id: r.school_id,
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            role: r.role.into(),
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(users))
}
