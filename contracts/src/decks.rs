//! Deck-management DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Expected JSON body for creating a deck.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDeck {
    pub title: String,
    pub description: Option<String>,
    /// Optional parent deck for nesting. The parent must exist in the same school.
    pub parent_id: Option<i64>,
}

/// Deserialize a field that distinguishes "absent" from "explicit null".
///
/// Used for partial-update semantics: omitting the field leaves it unchanged,
/// while sending `null` clears it.
fn deserialize_some_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// Fields that can be updated on a deck. All fields are optional.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateDeck {
    pub title: Option<String>,
    /// Move the deck to a new parent. Omit the field to leave parent unchanged.
    /// Include with any value (even null) to move the deck — null means root.
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub parent_id: Option<Option<i64>>,
    /// Assign a deck options preset. Omit to leave unchanged; null clears it.
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub options_id: Option<Option<i64>>,
    /// Update the deck description. Omit the field to leave unchanged.
    /// Include with `null` to clear the description.
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub description: Option<Option<String>>,
}

/// Query parameters for `DELETE /decks/{id}`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeleteDeckQuery {
    /// If true, also delete all descendant decks recursively.
    #[serde(default)]
    pub cascade: Option<bool>,
}

/// Expected JSON body for sharing a deck with a collaborator.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ShareDeck {
    /// The teacher to share the deck with.
    pub user_id: i64,
}

/// Expected JSON body for transferring deck ownership.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TransferOwner {
    /// The teacher who will become the new owner.
    pub user_id: i64,
}

/// Expected JSON body for adding a deck to a class.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddDeckToClass {
    pub class_id: i64,
}

/// A deck as returned to clients.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckResponse {
    pub id: i64,
    pub school_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub owner_email: String,
    pub owner_first_name: String,
    pub owner_last_name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    /// Card counts for the requesting student. Present only in list_decks.
    /// Not populated in get_deck, create_deck, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relearning_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
}

/// A collaborator on a shared deck.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CollaboratorResponse {
    pub user_id: i64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub shared_at: DateTime<Utc>,
}

/// A deck together with its collaborators and linked classes.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckDetailResponse {
    pub deck: DeckResponse,
    pub collaborators: Vec<CollaboratorResponse>,
    pub classes: Vec<ClassInfo>,
}

/// A minimal class reference.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClassInfo {
    pub id: i64,
    pub name: String,
}

/// Query parameters for `GET /decks/counts`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckCountsQuery {
    /// Optional: only return counts for this single deck.
    pub deck_id: Option<i64>,
}

/// Per-state card counts for a deck.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckCounts {
    pub deck_id: i64,
    pub new_count: i64,
    pub learning_count: i64,
    pub review_count: i64,
    pub relearning_count: i64,
    pub total_count: i64,
}

/// Response for `GET /decks/counts`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckCountsResponse {
    pub decks: Vec<DeckCounts>,
}
