//! Note DTOs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Expected JSON body for creating a note.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateNote {
    pub note_type_id: i64,
    pub fields: Map<String, Value>,
    /// The deck the note's generated cards go into (the "selected deck").
    pub deck_id: i64,
}

/// Fields that can be updated on a note.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateNote {
    pub note_type_id: Option<i64>,
    pub fields: Option<Map<String, Value>>,
}

/// Query parameters for `GET /notes`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListNotesQuery {
    /// Optional deck filter: only notes with at least one card in this deck.
    pub deck_id: Option<i64>,
}

/// A note as returned to clients.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteResponse {
    pub id: i64,
    pub note_type_id: i64,
    pub note_type_name: String,
    pub fields: Map<String, Value>,
    pub cards: Vec<CardSummary>,
    pub created_at: String,
}

/// A summary of one card generated from a note.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CardSummary {
    pub id: i64,
    pub template_id: i64,
    pub template_name: String,
    pub deck_id: i64,
    pub front: String,
    pub back: String,
}
