//! Deck-management DTOs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// How a deck resolves its daily new/review limits.
///
/// - `Preset`: use the shared preset's limit (default).
/// - `ThisDeck`: use this deck's own override value.
/// - `TodayOnly`: use this deck's override value, but only for today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LimitMode {
    Preset,
    ThisDeck,
    TodayOnly,
}

impl Default for LimitMode {
    fn default() -> Self {
        LimitMode::Preset
    }
}

/// Expected JSON body for creating a deck.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDeck {
    pub title: String,
    pub description: Option<String>,
    /// Optional parent deck for nesting. The parent must exist in the same school.
    pub parent_id: Option<i64>,
    /// Per-deck new-cards/day override mode (defaults to `preset`).
    #[serde(default)]
    pub new_per_day_mode: LimitMode,
    /// Per-deck max-reviews/day override mode (defaults to `preset`).
    #[serde(default)]
    pub review_per_day_mode: LimitMode,
    /// Override values for `this_deck`/`today_only` modes. Required when the
    /// corresponding mode is not `preset`; ignored otherwise.
    pub new_per_day_override: Option<i64>,
    pub review_per_day_override: Option<i64>,
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
    /// Update the deck description. Omit the field to leave unchanged.
    /// Include with `null` to clear the description.
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub description: Option<Option<String>>,
    /// Per-deck daily-limit override mode/values (see `LimitMode`). Omitting a
    /// mode leaves it unchanged; omitting an override value leaves it unchanged.
    pub new_per_day_mode: Option<LimitMode>,
    pub review_per_day_mode: Option<LimitMode>,
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub new_per_day_override: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_some_option")]
    pub review_per_day_override: Option<Option<i64>>,
}

/// Expected JSON body for assigning a deck's scheduling preset (US-4.11).
///
/// `options_id: 0` means "use the default" (the server stores `NULL`, which the
/// read path resolves back to the effective id 0); any non-zero id must be a
/// preset in the caller's school.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SetDeckOptions {
    pub options_id: i64,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The deck's effective scheduling preset id (the server resolves a null
    /// `decks.options_id` to the global default, id 0, before serialising — see
    /// US-4.9). Always a concrete, live preset id on the wire.
    pub options_id: i64,
    /// Per-deck daily-limit configuration (see `LimitMode`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_per_day_mode: Option<LimitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_per_day_mode: Option<LimitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_per_day_override: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_per_day_override: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_per_day_today_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_per_day_today_date: Option<NaiveDate>,
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
    /// Whether the requesting user may *study* this deck (vs. view it only as
    /// tree context). For students under subtree sharing (US-2.19), an ancestor
    /// of a granted deck is visible but not studyable. Owners/admins/collabs are
    /// always studyable. Defaults to true for backwards compatibility (older
    /// clients/fields omit it).
    #[serde(default = "default_studyable")]
    pub studyable: bool,
}

fn default_studyable() -> bool {
    true
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckCountsResponse {
    pub decks: Vec<DeckCounts>,
}
