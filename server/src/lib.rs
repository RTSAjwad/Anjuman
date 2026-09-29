//! Anjuman server — library target.
//!
//! This crate is split into a library (`lib.rs`) and a thin binary
//! (`main.rs`) so that integration tests (in `server/tests/`) can import the
//! router and `AppState` and exercise the HTTP API directly.
//!
//! The binary owns process concerns (logging, background tasks, binds a
//! listener); the library owns everything testable.

pub mod app;
pub mod auth;
pub mod db;
pub mod db_types;
pub mod deck_options;
pub mod note_types;
pub mod openapi;
pub mod routes;
pub mod state;

pub mod handlers {
    pub mod admin_users;
    pub mod analytics;
    pub mod card_browser;
    pub mod card_mod;
    pub mod classes;
    pub mod dashboard;
    pub mod deck_options_handler;
    pub mod decks;
    pub mod health;
    pub mod login;
    pub mod logout;
    pub mod me;
    pub mod note_types_handler;
    pub mod notes;
    pub mod preferences;
    pub mod reviews;
    pub mod search_users;
    pub mod study;
    pub mod users;
}

pub use app::app;
pub use state::AppState;
