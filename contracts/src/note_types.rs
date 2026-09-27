//! Note-type DTOs.

use serde::{Deserialize, Serialize};

/// Fields that can be updated on a note type.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateNoteType {
    pub name: Option<String>,
    pub field_names: Option<Vec<String>>,
    pub sort_field: Option<String>,
}

/// Expected JSON body for creating a card template.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTemplate {
    pub name: String,
    pub front_pattern: String,
    pub back_pattern: String,
}

/// Fields that can be updated on a card template.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateTemplate {
    pub name: Option<String>,
    pub front_pattern: Option<String>,
    pub back_pattern: Option<String>,
}

/// Expected JSON body for reordering templates.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReorderTemplates {
    pub template_ids: Vec<i64>,
}

/// A card template within a note type.
///
/// This is shared between the wire type ([`NoteTypeResponse`]) and the
/// server's internal note-type model, so it lives here in contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Template {
    /// Stable template identity (survives add/remove/reorder of siblings).
    pub id: i64,
    /// Display order within the note type (0-based).
    pub ord: i64,
    pub name: String,
    pub front_pattern: String,
    pub back_pattern: String,
}

/// A note type as returned to clients.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NoteTypeResponse {
    pub id: i64,
    pub name: String,
    pub field_names: Vec<String>,
    pub sort_field: String,
    pub templates: Vec<Template>,
    pub note_count: i64,
}
