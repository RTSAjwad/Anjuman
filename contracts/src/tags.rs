//! Tag DTOs (note-level, teacher/admin-authored).

use serde::{Deserialize, Serialize};

/// A single tag (the canonical row). The name may carry an Anki-style `::`
/// hierarchy (`biology::cells` is one tag named `biology::cells`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

/// Expected JSON body for adding tags to a note. Tags are submitted by name
/// (matching Anki's "type tags separated by spaces"); names are normalised
/// (trimmed) by the server and deduplicated.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddTags {
    /// Tags to add, by name. May include `::` hierarchy.
    pub tags: Vec<String>,
}

/// Expected JSON body for removing tags from a note (by name).
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RemoveTags {
    pub tags: Vec<String>,
}

/// Expected JSON body for renaming a tag (its `::` hierarchy may change with the
/// name). Only the canonical `tags` row is updated; `note_tags` keeps referencing
/// the same id.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RenameTag {
    pub name: String,
}
