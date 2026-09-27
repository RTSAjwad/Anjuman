// Users handler — registration and password management.
//
// This file handles user creation (registration) and provides the
// password hashing/verification utilities used by the login flow.

use axum::{Json, extract::State, http::StatusCode};

use chrono::Utc;

use anjuman_contracts::users::{CreateUser, User};

use crate::{db_types::DbUserRole, state::AppState};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
// Use the OsRng from password-hash's bundled rand_core to avoid version mismatch.
use password_hash::rand_core::OsRng;

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /users` — Create a new user (registration).
///
/// The password is hashed with Argon2 before storage. The plain-text
/// password is never written to the database or logged.
///
/// Returns 409 Conflict if a user with the given email already exists.
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUser>,
) -> Result<Json<User>, (StatusCode, String)> {
    // Hash the password with Argon2.
    //
    // Argon2 is the winner of the Password Hashing Competition and is
    // designed to be resistant to GPU/ASIC cracking. A random salt is
    // generated for each password, so two users with the same password
    // will have different hashes.
    let password_hash =
        hash_password(&body.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Insert the new user row.
    //
    // The `role` is a Postgres `user_role` enum. It is bound as its lowercase
    // string via `.as_str()` and cast to the enum on the SQL side, since the
    // compile-time `query!` macro can't map custom enum types directly.
    let result = sqlx::query!(
        r#"
        INSERT INTO users
        (
            school_id,
            email,
            password_hash,
            role,
            first_name,
            last_name,
            created_at
        )
        VALUES ($1, $2, $3, $4::text::user_role, $5, $6, $7)
        RETURNING id
        "#,
        body.school_id,
        body.email,
        password_hash,
        DbUserRole::from(body.role).as_str(),
        body.first_name,
        body.last_name,
        Utc::now()
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        // Postgres returns a UNIQUE constraint error when the email already
        // exists. We check for the word "UNIQUE" in the error message
        // because the exact error code varies.
        let msg = if e.to_string().contains("UNIQUE") {
            "A user with that email already exists".into()
        } else {
            format!("Database error: {e}")
        };
        (StatusCode::CONFLICT, msg)
    })?;

    // `RETURNING id` returns the `BIGINT` identity value that Postgres
    // auto-generated for the new row.
    Ok(Json(User {
        id: result.id,
        email: body.email,
        first_name: body.first_name,
        last_name: body.last_name,
        role: body.role,
        school_id: body.school_id,
    }))
}

// ---------------------------------------------------------------------------
// Password helpers
// ---------------------------------------------------------------------------

/// Hash a plain-text password using Argon2.
///
/// Returns the encoded hash string suitable for database storage.
/// The hash includes the algorithm parameters and the random salt,
/// so everything needed for verification is in a single string.
pub fn hash_password(password: &str) -> Result<String, String> {
    // Generate a cryptographically random salt.
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password with Argon2 using default parameters
    // (memory: 19 MiB, iterations: 2, parallelism: 1).
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Password hashing failed: {e}"))
}

/// Verify a plain-text password against an Argon2 hash.
///
/// Returns:
///   `Ok(true)`  — password matches
///   `Ok(false)` — password does not match (wrong password)
///   `Err(...)`  — the stored hash is malformed (shouldn't happen)
///
/// This runs in constant time to prevent timing attacks.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::password_hash::PasswordVerifier;

    // Parse the stored hash string (which includes algorithm parameters
    // and the salt) back into a structured `PasswordHash`.
    let parsed_hash = argon2::password_hash::PasswordHash::new(hash)
        .map_err(|e| format!("Invalid password hash: {e}"))?;

    // Verify the password against the hash.
    //
    // Argon2's `verify_password` is constant-time: it doesn't short-circuit
    // on the first wrong byte, which prevents attackers from measuring
    // response times to guess the password character by character.
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map(|_| true)
        .or_else(|e| match e {
            argon2::password_hash::Error::Password => Ok(false),
            other => Err(format!("Password verification error: {other}")),
        })
}
