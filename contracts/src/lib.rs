//! `anjuman_contracts` — the single source of truth for the DTOs that cross
//! the network between the Anjuman client core and the Axum server.
//!
//! Every struct and enum here is a wire type: a request body, a response body,
//! or a shared value (like [`UserRole`]) that appears in both. The server
//! serializes these as JSON; the client core deserializes them.
//!
//! Types that are *only* used server-side (database rows, rendering helpers,
//! auth internals) deliberately live in the server crate, not here.
//!
//! # Features
//!
//! - `openapi` — enables `utoipa` (`ToSchema`) derives so the server can
//!   generate an OpenAPI document directly from these types.

pub mod analytics;
pub mod auth;
pub mod cards;
pub mod classes;
pub mod deck_options;
pub mod decks;
pub mod health;
pub mod note_types;
pub mod notes;
pub mod reviews;
pub mod shared;
pub mod study;
pub mod users;

pub use shared::{MessageResponse, UserRole};
