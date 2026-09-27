//! Study-session DTOs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::reviews::ReviewedCardState;

/// A card presented during study.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudyCard {
    pub card_id: i64,
    /// The note this card was generated from (for note-level actions like
    /// bury/suspend).
    pub note_id: i64,
    pub front: String,
    pub back: String,
    pub state: String,
    pub due_at: Option<i64>,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    /// Anki-style card flag (0-7). 0 means no flag.
    pub flag: i64,
    /// 1 if suspended, 0 otherwise.
    pub suspended: i64,
    /// Unix seconds when this card was buried; null if not currently buried.
    /// A non-null value on/before today's start auto-expires (unburied).
    pub buried_at: Option<i64>,
    /// Why this card is buried (`user_card`, `sibling_new`, …); null if not buried.
    pub bury_reason: Option<String>,
    /// Current position in the learning/relearning steps list (0-based).
    pub step_index: i64,
    /// Predicted interval (in seconds) until next review for each rating.
    /// Key: "1" (Again), "2" (Hard), "3" (Good), "4" (Easy).
    pub predicted_interval: Option<HashMap<String, i64>>,
}

/// Limit-aware per-state counts for a student's deck study.
///
/// `new_count` and `review_count` are clamped to the remaining daily budget,
/// so they represent what the student can still actually do today. Learning
/// and relearning are exempt from limits (in-progress steps always return).
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudyCounts {
    pub new_count: i64,
    pub learning_count: i64,
    pub review_count: i64,
    pub relearning_count: i64,
}

/// The unified response for both GET (start) and POST (advance).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudyAdvance {
    /// The next card to study, or null when nothing is currently due.
    pub next_card: Option<StudyCard>,
    /// On POST, the post-review state of the card that was just reviewed.
    /// On GET, null.
    pub reviewed_card: Option<ReviewedCardState>,
    pub counts: StudyCounts,
    pub deck_id: i64,
    pub deck_title: String,
}

/// Request body for `POST /decks/:id/study`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudyAdvanceBody {
    pub card_id: i64,
    pub rating: i32,
    pub response_time_ms: Option<i64>,
}
