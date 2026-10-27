//! Card and card-browser DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::tags::Tag;

/// Request body for rescheduling a card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RescheduleBody {
    /// Absolute due timestamp.
    pub due_at: Option<DateTime<Utc>>,
    /// Relative due offset in days from now (alternative to `due_at`).
    pub days: Option<i64>,
}

/// Request body for moving a card to another deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MoveCardBody {
    /// The target deck to move the card into.
    pub deck_id: i64,
}

/// Response after moving a card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MoveCardResponse {
    pub card_id: i64,
    pub deck_id: i64,
}

/// Request body for repositioning a new card in the new-card queue (Anki's
/// "Reposition"). `position` is the target collection-wide new-card queue
/// ordinal (0 = front of the queue). Distinct from reschedule (absolute due).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RepositionBody {
    pub position: i64,
}

/// Response after repositioning a new card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RepositionResponse {
    pub card_id: i64,
    pub position: i64,
}

/// Query parameters for unburying a card/note.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UnburyQuery {
    pub reason: Option<String>,
}

/// Response after a card-level modification (suspend/unsuspend/bury/unbury).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardModResponse {
    pub card_id: i64,
    pub suspended: bool,
    pub buried_at: Option<DateTime<Utc>>,
    /// Reason the card is buried (`user_card`, `sibling_new`, …); null if not buried.
    pub bury_reason: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub state: String,
}

/// Response after a note-level modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteModResponse {
    pub note_id: i64,
    pub cards_affected: i64,
    /// IDs of the cards that were affected.
    pub card_ids: Vec<i64>,
    /// True if suspended (suspend operation).
    pub suspended: bool,
    /// When cards were buried; null if not buried (bury operation).
    pub buried_at: Option<DateTime<Utc>>,
    /// Reason cards were buried; null if not buried (bury operation).
    pub bury_reason: Option<String>,
}

/// Query parameters for `GET /cards`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Comma-separated card statuses: "buried", "suspended" (OR'd). Empty = no
    /// status filter.
    #[serde(default)]
    pub status: String,
    /// Comma-separated tag names (note-scoped tags; cards whose note has any of
    /// these tags). Empty = no tag filter.
    #[serde(default)]
    pub tag: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The note's tags (shared note taxonomy), ordered by name.
    pub tags: Vec<Tag>,
    /// The resolved sort-field value (Anki's "Sort Field"): the note type's
    /// `sort_field` (fallback: first field) resolved against the note's fields.
    pub sort_field: Option<String>,
    /// The card's deck's full ancestor path, top-down, joined with `::` (e.g.
    /// `Biology 101::Cell Biology`). For a root deck this equals `deck_title`.
    pub deck_path: String,
    pub state: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub reps: Option<i64>,
    pub lapses: Option<i64>,
    /// Anki-style card flag (0-7). 0 means no flag.
    pub flag: Option<i64>,
    /// True if suspended (student view only).
    pub suspended: Option<bool>,
    /// When the card was buried (student view only); null if not buried.
    pub buried_at: Option<DateTime<Utc>>,
    /// Why the card is buried; null if not buried (student view only).
    pub bury_reason: Option<String>,
    /// True if *all* of the note's cards are suspended for this student.
    pub note_suspended: bool,
    /// True if *all* of the note's cards are buried for this student.
    pub note_buried: bool,
    /// The number of cards generated by this note (all templates, across decks).
    pub card_count: i64,
    /// FSRS-predicted interval (in seconds) per rating for this card's grade
    /// dialog — same shape as the study screen's `StudyCard.predicted_interval`
    /// (string keys "1".."4"). `None` when no prediction applies.
    pub predicted_interval: Option<std::collections::HashMap<String, i64>>,
    pub created_at: DateTime<Utc>,
    /// The stored `cards.position` (the card's place in the collection-wide
    /// new-card queue). `Some` only when the card is **new** (matches Anki's
    /// Due column, which shows a position for new cards and an interval for
    /// scheduled ones); `None` for review/learning/relearning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

/// A page of card-browser results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardBrowserPage {
    pub cards: Vec<CardBrowserResponse>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}
