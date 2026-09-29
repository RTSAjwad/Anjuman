//! Deck-options (scheduling preset) DTOs.

use serde::{Deserialize, Serialize};

/// How new cards are gathered from a deck (US-2.8).
///
/// Mirrors Anki's "New card gather order". `Deck` gathers subdecks in order
/// (each in ascending position); `DeckThenRandomNotes` keeps subdeck order but
/// gathers randomly selected notes within each subdeck; `Ascending`/`Descending`
/// order by `cards.position`; `RandomNotes`/`RandomCards` use a deterministic
/// per-student-day seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum NewGatherOrder {
    Deck,
    DeckThenRandomNotes,
    Ascending,
    Descending,
    RandomNotes,
    RandomCards,
}

impl Default for NewGatherOrder {
    fn default() -> Self {
        NewGatherOrder::Deck
    }
}

/// How gathered new cards are sorted (US-2.9).
///
/// Mirrors Anki's "New card sort order". Sorting happens *after* gathering, so
/// it reorders the already-gathered new-card set. Variants use `template.ord`
/// for card-type ordering and a deterministic per-student-day seed for the
/// random variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum NewSortOrder {
    CardTypeThenGathered,
    Gathered,
    CardTypeThenRandom,
    RandomNoteThenCardType,
    Random,
}

impl Default for NewSortOrder {
    fn default() -> Self {
        NewSortOrder::CardTypeThenGathered
    }
}

/// When new cards are shown relative to review cards (US-2.10).
///
/// Mirrors Anki's three options. `Mix` is Anki's `Intersperser`: a stateless
/// ratio-based even distribution of the two queues (not a due-date merge),
/// expressible in our single-card model from the day counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum NewReviewOrder {
    Mix,
    Before,
    After,
}

impl Default for NewReviewOrder {
    fn default() -> Self {
        NewReviewOrder::Mix
    }
}

/// When interday (re)learning cards are shown relative to review cards (US-2.11).
///
/// Mirrors Anki's three options; `Mix` uses the same stateless `Intersperser`
/// as `NewReviewOrder::Mix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum InterdayOrder {
    Mix,
    Before,
    After,
}

impl Default for InterdayOrder {
    fn default() -> Self {
        InterdayOrder::Mix
    }
}

/// How review cards are sorted (US-2.12).
///
/// Mirrors the 13 in-app review-sort options. `interval` maps to FSRS
/// `stability`; `easy`/`difficult` map to FSRS `difficulty`; `retrievability`
/// uses `fsrs::current_retrievability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ReviewSortOrder {
    DueThenRandom,
    DueThenDeck,
    DeckThenDue,
    AscendingInterval,
    DescendingInterval,
    EasyFirst,
    DifficultFirst,
    AscendingRetrievability,
    DescendingRetrievability,
    RelativeOverdueness,
    Random,
    OrderAdded,
    LatestAddedFirst,
}

impl Default for ReviewSortOrder {
    fn default() -> Self {
        ReviewSortOrder::DueThenRandom
    }
}

/// The action taken when a card reaches the leech threshold.
///
/// Mirrors Anki's leech action. `SuspendCard` also tags the note in Anki, but
/// this repo has no tag system yet (see `ROADMAP.md` stage 6); until then the
/// tag half is omitted and the note is marked via `notes.leech_tagged_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LeechAction {
    TagOnly,
    SuspendCard,
}

impl Default for LeechAction {
    fn default() -> Self {
        LeechAction::SuspendCard
    }
}

/// The action to apply after the question is shown and the auto-advance timer
/// elapses (US-2.14). Selection-only: the client performs the action; the
/// server only persists the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AutoAdvanceQuestionAction {
    ShowAnswer,
    ShowCard,
}

impl Default for AutoAdvanceQuestionAction {
    fn default() -> Self {
        AutoAdvanceQuestionAction::ShowAnswer
    }
}

/// The action to apply after the answer is shown and the auto-advance timer
/// elapses (US-2.14). Selection-only: the client performs the action; the
/// server only persists the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AutoAdvanceAnswerAction {
    BuryCard,
    AnswerAgain,
    AnswerGood,
    AnswerHard,
    ShowReminder,
}

impl Default for AutoAdvanceAnswerAction {
    fn default() -> Self {
        AutoAdvanceAnswerAction::BuryCard
    }
}

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
    #[serde(default = "default_leech_threshold")]
    pub leech_threshold: i64,
    #[serde(default)]
    pub leech_action: LeechAction,
    #[serde(default)]
    pub new_gather_order: NewGatherOrder,
    #[serde(default)]
    pub new_sort_order: NewSortOrder,
    #[serde(default)]
    pub new_review_order: NewReviewOrder,
    #[serde(default)]
    pub interday_order: InterdayOrder,
    #[serde(default)]
    pub review_sort_order: ReviewSortOrder,
    // --- Selection-only (client-side behaviour) options (US-2.14) ---
    #[serde(default)]
    pub show_on_screen_timer: bool,
    #[serde(default)]
    pub stop_timer_on_answer: bool,
    #[serde(default)]
    pub dont_play_audio_automatically: bool,
    #[serde(default)]
    pub skip_question_when_replaying_answer: bool,
    #[serde(default)]
    pub auto_advance_seconds_show_question: f64,
    #[serde(default)]
    pub auto_advance_seconds_show_answer: f64,
    #[serde(default = "default_wait_for_audio")]
    pub auto_advance_wait_for_audio: bool,
    #[serde(default)]
    pub auto_advance_question_action: AutoAdvanceQuestionAction,
    #[serde(default)]
    pub auto_advance_answer_action: AutoAdvanceAnswerAction,
    /// Maximum recorded answer time, in seconds (default 60; validated 1..=7200).
    #[serde(default = "default_maximum_answer_seconds")]
    pub maximum_answer_seconds: i64,
    /// Maximum review interval, in days (default 36500; validated 1..=36500).
    #[serde(default = "default_maximum_interval")]
    pub maximum_interval: i64,
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
    pub leech_threshold: Option<i64>,
    pub leech_action: Option<LeechAction>,
    pub new_gather_order: Option<NewGatherOrder>,
    pub new_sort_order: Option<NewSortOrder>,
    pub new_review_order: Option<NewReviewOrder>,
    pub interday_order: Option<InterdayOrder>,
    pub review_sort_order: Option<ReviewSortOrder>,
    // --- Selection-only (client-side behaviour) options (US-2.14) ---
    pub show_on_screen_timer: Option<bool>,
    pub stop_timer_on_answer: Option<bool>,
    pub dont_play_audio_automatically: Option<bool>,
    pub skip_question_when_replaying_answer: Option<bool>,
    pub auto_advance_seconds_show_question: Option<f64>,
    pub auto_advance_seconds_show_answer: Option<f64>,
    pub auto_advance_wait_for_audio: Option<bool>,
    pub auto_advance_question_action: Option<AutoAdvanceQuestionAction>,
    pub auto_advance_answer_action: Option<AutoAdvanceAnswerAction>,
    pub maximum_answer_seconds: Option<i64>,
    pub maximum_interval: Option<i64>,
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
    pub leech_threshold: i64,
    pub leech_action: LeechAction,
    pub new_gather_order: NewGatherOrder,
    pub new_sort_order: NewSortOrder,
    pub new_review_order: NewReviewOrder,
    pub interday_order: InterdayOrder,
    pub review_sort_order: ReviewSortOrder,
    // --- Selection-only (client-side behaviour) options (US-2.14) ---
    pub show_on_screen_timer: bool,
    pub stop_timer_on_answer: bool,
    pub dont_play_audio_automatically: bool,
    pub skip_question_when_replaying_answer: bool,
    pub auto_advance_seconds_show_question: f64,
    pub auto_advance_seconds_show_answer: f64,
    pub auto_advance_wait_for_audio: bool,
    pub auto_advance_question_action: AutoAdvanceQuestionAction,
    pub auto_advance_answer_action: AutoAdvanceAnswerAction,
    pub maximum_answer_seconds: i64,
    pub maximum_interval: i64,
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

fn default_leech_threshold() -> i64 {
    8
}

fn default_wait_for_audio() -> bool {
    true
}

fn default_maximum_answer_seconds() -> i64 {
    60
}

fn default_maximum_interval() -> i64 {
    36500
}
