//! Note DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::tags::Tag;

/// Expected JSON body for creating a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateNote {
    pub note_type_id: i64,
    pub fields: Map<String, Value>,
    /// The deck the note's generated cards go into (the "selected deck").
    pub deck_id: i64,
}

/// Fields that can be updated on a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateNote {
    pub note_type_id: Option<i64>,
    pub fields: Option<Map<String, Value>>,
}

/// Query parameters for `GET /notes` (paginated + filterable, mirroring
/// `CardBrowserQuery` for the merged browser).
///
/// `deck_id`/`note_type_id` are comma-separated id lists (multi-select); `sort`
/// is a note-appropriate key (`created_at` | `note_type` | `deck` | `question`);
/// `q` is a text search over field content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListNotesQuery {
    /// Comma-separated deck ids (e.g. "1,2").
    #[serde(default)]
    pub deck_id: String,
    /// Comma-separated note-type ids (e.g. "1,2").
    #[serde(default)]
    pub note_type_id: String,
    /// Free-text search over `fields_json` content.
    pub q: Option<String>,
    /// Sort key: `created_at` (default) | `note_type` | `deck` | `question`.
    #[serde(default = "default_notes_sort")]
    pub sort: String,
    #[serde(default = "default_notes_page")]
    pub page: i64,
    #[serde(default = "default_notes_per_page")]
    pub per_page: i64,
}

fn default_notes_sort() -> String {
    "created_at".to_string()
}

fn default_notes_page() -> i64 {
    1
}

fn default_notes_per_page() -> i64 {
    50
}

/// A page of note-browser results (paginated `GET /notes` response).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteBrowserPage {
    pub notes: Vec<NoteResponse>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

/// A note as returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteResponse {
    pub id: i64,
    pub note_type_id: i64,
    pub note_type_name: String,
    pub fields: Map<String, Value>,
    pub cards: Vec<CardSummary>,
    /// This note's tags (shared note taxonomy).
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
}

/// A summary of one card generated from a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardSummary {
    pub id: i64,
    pub template_id: i64,
    pub template_name: String,
    pub deck_id: i64,
    pub front: String,
    pub back: String,
}
