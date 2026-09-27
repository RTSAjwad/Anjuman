//! Card and card-browser DTOs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Request body for rescheduling a card.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RescheduleBody {
    /// Absolute due timestamp (Unix epoch seconds).
    pub due_at: Option<i64>,
    /// Relative due offset in days from now (alternative to `due_at`).
    pub days: Option<i64>,
}

/// Request body for moving a card to another deck.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MoveCardBody {
    /// The target deck to move the card into.
    pub deck_id: i64,
}

/// Response after moving a card.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MoveCardResponse {
    pub card_id: i64,
    pub deck_id: i64,
}

/// Query parameters for unburying a card/note.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UnburyQuery {
    pub reason: Option<String>,
}

/// Response after a card-level modification (suspend/unsuspend/bury/unbury).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardModResponse {
    pub card_id: i64,
    pub suspended: i64,
    pub buried_at: Option<i64>,
    /// Reason the card is buried (`user_card`, `sibling_new`, …); null if not buried.
    pub bury_reason: Option<String>,
    pub due_at: Option<i64>,
    pub state: String,
}

/// Response after a note-level modification.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteModResponse {
    pub note_id: i64,
    pub cards_affected: i64,
    /// IDs of the cards that were affected.
    pub card_ids: Vec<i64>,
    /// 1 if suspended, 0 otherwise (suspend operation).
    pub suspended: i64,
    /// Unix seconds when cards were buried; null if not buried (bury operation).
    pub buried_at: Option<i64>,
    /// Reason cards were buried; null if not buried (bury operation).
    pub bury_reason: Option<String>,
}

/// Query parameters for `GET /cards`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardBrowserQuery {
    /// Comma-separated deck IDs, e.g. "1,2,3"
    #[serde(default)]
    pub deck_id: String,
    /// Comma-separated note type IDs, e.g. "1,2"
    #[serde(default)]
    pub note_type_id: String,
    pub q: Option<String>,
    /// Comma-separated states, e.g. "review,learning"
    #[serde(default)]
    pub state: String,
    /// Comma-separated flag values, e.g. "1,3" for red and green.
    #[serde(default)]
    pub flag: String,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_sort() -> String {
    "created_at".to_string()
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    50
}

/// A single card in the browser view.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardBrowserResponse {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub deck_title: String,
    pub template_id: i64,
    pub front: String,
    pub back: String,
    pub note_type_name: String,
    pub note_type_id: i64,
    pub template_name: String,
    pub fields: Map<String, Value>,
    pub state: Option<String>,
    pub due_at: Option<i64>,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub reps: Option<i64>,
    pub lapses: Option<i64>,
    /// Anki-style card flag (0-7). 0 means no flag.
    pub flag: Option<i64>,
    /// 1 if suspended, 0 otherwise (student view only).
    pub suspended: Option<i64>,
    /// Unix seconds when the card was buried (student view only); null if not buried.
    pub buried_at: Option<i64>,
    /// Why the card is buried; null if not buried (student view only).
    pub bury_reason: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_card_position: Option<i64>,
}

/// A page of card-browser results.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardBrowserPage {
    pub cards: Vec<CardBrowserResponse>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}
