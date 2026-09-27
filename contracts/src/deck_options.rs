//! Deck-options (scheduling preset) DTOs.

use serde::{Deserialize, Serialize};

/// Expected JSON body for creating a deck options preset.
///
/// Learning/relearning steps are submitted as Anki-style strings (e.g.
/// `"1m 10m"`) and the server parses them into seconds.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateDeckOptions {
    pub name: String,
    /// Anki-style step string, e.g. "1m 10m". Parsed into seconds.
    #[serde(default = "default_learning_steps")]
    pub learning_steps: String,
    #[serde(default = "default_relearning_steps")]
    pub relearning_steps: String,
    #[serde(default = "default_retention")]
    pub desired_retention: f64,
    #[serde(default)]
    pub bury_new: bool,
    #[serde(default)]
    pub bury_review: bool,
    #[serde(default)]
    pub bury_interday: bool,
    #[serde(default = "default_new_per_day")]
    pub new_per_day: i64,
    #[serde(default = "default_review_per_day")]
    pub review_per_day: i64,
}

/// Fields that can be updated on a deck options preset.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateDeckOptions {
    pub name: Option<String>,
    pub learning_steps: Option<String>,
    pub relearning_steps: Option<String>,
    pub desired_retention: Option<f64>,
    pub bury_new: Option<bool>,
    pub bury_review: Option<bool>,
    pub bury_interday: Option<bool>,
    pub new_per_day: Option<i64>,
    pub review_per_day: Option<i64>,
}

/// A deck options preset as returned to clients.
///
/// Steps are stored normalized (one row per step) and re-assembled here into
/// `Vec<i64>` (seconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeckOptions {
    pub id: i64,
    pub school_id: i64,
    pub name: String,
    pub learning_steps: Vec<i64>,
    pub relearning_steps: Vec<i64>,
    pub desired_retention: f64,
    pub bury_new: bool,
    pub bury_review: bool,
    pub bury_interday: bool,
    pub new_per_day: i64,
    pub review_per_day: i64,
}

fn default_learning_steps() -> String {
    "1m 10m".to_string()
}

fn default_relearning_steps() -> String {
    "10m".to_string()
}

fn default_retention() -> f64 {
    0.9
}

fn default_new_per_day() -> i64 {
    20
}

fn default_review_per_day() -> i64 {
    200
}
