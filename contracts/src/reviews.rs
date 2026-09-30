//! Review DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for submitting a review.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SubmitReview {
    pub card_id: i64,
    /// Rating 1-4 (Again / Hard / Good / Easy).
    pub rating: i32,
    pub response_time_ms: Option<i64>,
}

/// Response after a review is applied.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReviewResponse {
    pub card_id: i64,
    pub state: String,
    pub due_at: Option<DateTime<Utc>>,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    /// The exact time (in seconds) until this card is due again, regardless
    /// of whether it came from an FSRS interval or a learning/relearning step.
    pub applied_interval_secs: i64,
}

/// The post-review scheduling state of a card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReviewedCardState {
    pub card_id: i64,
    pub state: String,
    pub due_at: Option<DateTime<Utc>>,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub step_index: i64,
    /// The exact time (in seconds) until this card is due again, regardless
    /// of whether it came from an FSRS interval or a learning/relearning step.
    pub applied_interval_secs: i64,
}

/// Request body for setting a card flag.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SetFlag {
    /// Flag value 0-7. 0 clears the flag.
    pub flag: i32,
}

/// Response after setting a card flag.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FlagResponse {
    pub card_id: i64,
    pub flag: i64,
}
