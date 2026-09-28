//! Server-local mirror enums for the Postgres `ENUM` column types.
//!
//! sqlx's compile-time `query!`/`query_as!` macros cannot map a custom Postgres
//! `ENUM` directly to a Rust type (see `TypeChecking::return_type_for_id`,
//! which only knows a fixed built-in list). So enum columns are handled with:
//!
//! - **Reads** — an explicit type override in the SQL, e.g.
//!   `role as "role!: DbUserRole"`. sqlx decodes the enum into the `Db*` type
//!   here (thanks to `#[derive(sqlx::Type)]`), keeping the column `NOT NULL`
//!   and avoiding any `.parse()` round-trip.
//! - **Writes** — the enum's lowercase string via `.as_str()`, cast on the SQL
//!   side (`$1::text::user_role`), since the compile-time macros can't accept
//!   a custom enum as a binding directly.
//!
//! These enums deliberately mirror (not reuse) the `anjuman_contracts` enums so
//! that `contracts` stays free of any `sqlx` dependency. Convert to/from the
//! contract types at the handler boundary.

use sqlx::Type;

/// Mirrors the `user_role` enum (`admin`, `teacher`, `student`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum DbUserRole {
    Admin,
    Teacher,
    Student,
}

/// Mirrors the `membership_role` enum (`teacher`, `student`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "membership_role", rename_all = "lowercase")]
pub enum DbMembershipRole {
    Teacher,
    Student,
}

/// Mirrors the `card_state` enum (`new`, `learning`, `review`, `relearning`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "card_state", rename_all = "lowercase")]
pub enum DbCardState {
    New,
    Learning,
    Review,
    Relearning,
}

/// Mirrors the `step_kind` enum (`learning`, `relearning`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "step_kind", rename_all = "lowercase")]
pub enum DbStepKind {
    Learning,
    Relearning,
}

/// Mirrors the `leech_action` enum (`tag_only`, `suspend_card`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "leech_action", rename_all = "snake_case")]
pub enum DbLeechAction {
    TagOnly,
    SuspendCard,
}

/// Mirrors the `limit_mode` enum (`preset`, `this_deck`, `today_only`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "limit_mode", rename_all = "snake_case")]
pub enum DbLimitMode {
    Preset,
    ThisDeck,
    TodayOnly,
}

/// Mirrors the `new_gather_order` enum (`deck`, `deck_then_random_notes`,
/// `ascending`, `descending`, `random_notes`, `random_cards`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "new_gather_order", rename_all = "snake_case")]
pub enum DbNewGatherOrder {
    Deck,
    DeckThenRandomNotes,
    Ascending,
    Descending,
    RandomNotes,
    RandomCards,
}

/// Mirrors the `new_sort_order` enum (`card_type_then_gathered`, `gathered`,
/// `card_type_then_random`, `random_note_then_card_type`, `random`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "new_sort_order", rename_all = "snake_case")]
pub enum DbNewSortOrder {
    CardTypeThenGathered,
    Gathered,
    CardTypeThenRandom,
    RandomNoteThenCardType,
    Random,
}

// ---------------------------------------------------------------------------
// Conversions (server-local DB enums <-> contract enum / application strings)
// ---------------------------------------------------------------------------

impl From<anjuman_contracts::UserRole> for DbUserRole {
    fn from(r: anjuman_contracts::UserRole) -> Self {
        match r {
            anjuman_contracts::UserRole::Admin => DbUserRole::Admin,
            anjuman_contracts::UserRole::Teacher => DbUserRole::Teacher,
            anjuman_contracts::UserRole::Student => DbUserRole::Student,
        }
    }
}

impl From<DbUserRole> for anjuman_contracts::UserRole {
    fn from(r: DbUserRole) -> Self {
        match r {
            DbUserRole::Admin => anjuman_contracts::UserRole::Admin,
            DbUserRole::Teacher => anjuman_contracts::UserRole::Teacher,
            DbUserRole::Student => anjuman_contracts::UserRole::Student,
        }
    }
}

impl From<DbMembershipRole> for anjuman_contracts::UserRole {
    fn from(r: DbMembershipRole) -> Self {
        match r {
            DbMembershipRole::Teacher => anjuman_contracts::UserRole::Teacher,
            DbMembershipRole::Student => anjuman_contracts::UserRole::Student,
        }
    }
}

impl From<DbUserRole> for DbMembershipRole {
    fn from(r: DbUserRole) -> Self {
        match r {
            DbUserRole::Teacher => DbMembershipRole::Teacher,
            // Students and admins both fall through to Student; `admin` is
            // rejected by callers before this conversion is used.
            _ => DbMembershipRole::Student,
        }
    }
}

impl DbUserRole {
    /// The role as a lowercase string (e.g. `"teacher"`), used to bind enum
    /// columns via a `::user_role` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbUserRole::Admin => "admin",
            DbUserRole::Teacher => "teacher",
            DbUserRole::Student => "student",
        }
    }
}

impl DbMembershipRole {
    /// The role as a lowercase string (e.g. `"student"`).
    pub fn as_str(self) -> &'static str {
        match self {
            DbMembershipRole::Teacher => "teacher",
            DbMembershipRole::Student => "student",
        }
    }
}

impl From<anjuman_contracts::deck_options::LeechAction> for DbLeechAction {
    fn from(a: anjuman_contracts::deck_options::LeechAction) -> Self {
        match a {
            anjuman_contracts::deck_options::LeechAction::TagOnly => DbLeechAction::TagOnly,
            anjuman_contracts::deck_options::LeechAction::SuspendCard => DbLeechAction::SuspendCard,
        }
    }
}

impl From<DbLeechAction> for anjuman_contracts::deck_options::LeechAction {
    fn from(a: DbLeechAction) -> Self {
        match a {
            DbLeechAction::TagOnly => anjuman_contracts::deck_options::LeechAction::TagOnly,
            DbLeechAction::SuspendCard => {
                anjuman_contracts::deck_options::LeechAction::SuspendCard
            }
        }
    }
}

impl DbLeechAction {
    /// The action as a lowercase string (e.g. `"suspend_card"`), for binding
    /// enum columns via a `::leech_action` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbLeechAction::TagOnly => "tag_only",
            DbLeechAction::SuspendCard => "suspend_card",
        }
    }
}

impl From<anjuman_contracts::decks::LimitMode> for DbLimitMode {
    fn from(m: anjuman_contracts::decks::LimitMode) -> Self {
        match m {
            anjuman_contracts::decks::LimitMode::Preset => DbLimitMode::Preset,
            anjuman_contracts::decks::LimitMode::ThisDeck => DbLimitMode::ThisDeck,
            anjuman_contracts::decks::LimitMode::TodayOnly => DbLimitMode::TodayOnly,
        }
    }
}

impl From<DbLimitMode> for anjuman_contracts::decks::LimitMode {
    fn from(m: DbLimitMode) -> Self {
        match m {
            DbLimitMode::Preset => anjuman_contracts::decks::LimitMode::Preset,
            DbLimitMode::ThisDeck => anjuman_contracts::decks::LimitMode::ThisDeck,
            DbLimitMode::TodayOnly => anjuman_contracts::decks::LimitMode::TodayOnly,
        }
    }
}

impl DbLimitMode {
    /// The mode as a lowercase string, for binding enum columns via a
    /// `::limit_mode` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbLimitMode::Preset => "preset",
            DbLimitMode::ThisDeck => "this_deck",
            DbLimitMode::TodayOnly => "today_only",
        }
    }
}

impl From<anjuman_contracts::deck_options::NewGatherOrder> for DbNewGatherOrder {
    fn from(o: anjuman_contracts::deck_options::NewGatherOrder) -> Self {
        match o {
            anjuman_contracts::deck_options::NewGatherOrder::Deck => DbNewGatherOrder::Deck,
            anjuman_contracts::deck_options::NewGatherOrder::DeckThenRandomNotes => {
                DbNewGatherOrder::DeckThenRandomNotes
            }
            anjuman_contracts::deck_options::NewGatherOrder::Ascending => DbNewGatherOrder::Ascending,
            anjuman_contracts::deck_options::NewGatherOrder::Descending => DbNewGatherOrder::Descending,
            anjuman_contracts::deck_options::NewGatherOrder::RandomNotes => DbNewGatherOrder::RandomNotes,
            anjuman_contracts::deck_options::NewGatherOrder::RandomCards => DbNewGatherOrder::RandomCards,
        }
    }
}

impl From<DbNewGatherOrder> for anjuman_contracts::deck_options::NewGatherOrder {
    fn from(o: DbNewGatherOrder) -> Self {
        match o {
            DbNewGatherOrder::Deck => anjuman_contracts::deck_options::NewGatherOrder::Deck,
            DbNewGatherOrder::DeckThenRandomNotes => {
                anjuman_contracts::deck_options::NewGatherOrder::DeckThenRandomNotes
            }
            DbNewGatherOrder::Ascending => anjuman_contracts::deck_options::NewGatherOrder::Ascending,
            DbNewGatherOrder::Descending => anjuman_contracts::deck_options::NewGatherOrder::Descending,
            DbNewGatherOrder::RandomNotes => anjuman_contracts::deck_options::NewGatherOrder::RandomNotes,
            DbNewGatherOrder::RandomCards => anjuman_contracts::deck_options::NewGatherOrder::RandomCards,
        }
    }
}

impl DbNewGatherOrder {
    /// The order as a lowercase string, for binding enum columns via a
    /// `::new_gather_order` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbNewGatherOrder::Deck => "deck",
            DbNewGatherOrder::DeckThenRandomNotes => "deck_then_random_notes",
            DbNewGatherOrder::Ascending => "ascending",
            DbNewGatherOrder::Descending => "descending",
            DbNewGatherOrder::RandomNotes => "random_notes",
            DbNewGatherOrder::RandomCards => "random_cards",
        }
    }
}

impl From<anjuman_contracts::deck_options::NewSortOrder> for DbNewSortOrder {
    fn from(o: anjuman_contracts::deck_options::NewSortOrder) -> Self {
        match o {
            anjuman_contracts::deck_options::NewSortOrder::CardTypeThenGathered => {
                DbNewSortOrder::CardTypeThenGathered
            }
            anjuman_contracts::deck_options::NewSortOrder::Gathered => DbNewSortOrder::Gathered,
            anjuman_contracts::deck_options::NewSortOrder::CardTypeThenRandom => {
                DbNewSortOrder::CardTypeThenRandom
            }
            anjuman_contracts::deck_options::NewSortOrder::RandomNoteThenCardType => {
                DbNewSortOrder::RandomNoteThenCardType
            }
            anjuman_contracts::deck_options::NewSortOrder::Random => DbNewSortOrder::Random,
        }
    }
}

impl From<DbNewSortOrder> for anjuman_contracts::deck_options::NewSortOrder {
    fn from(o: DbNewSortOrder) -> Self {
        match o {
            DbNewSortOrder::CardTypeThenGathered => {
                anjuman_contracts::deck_options::NewSortOrder::CardTypeThenGathered
            }
            DbNewSortOrder::Gathered => anjuman_contracts::deck_options::NewSortOrder::Gathered,
            DbNewSortOrder::CardTypeThenRandom => {
                anjuman_contracts::deck_options::NewSortOrder::CardTypeThenRandom
            }
            DbNewSortOrder::RandomNoteThenCardType => {
                anjuman_contracts::deck_options::NewSortOrder::RandomNoteThenCardType
            }
            DbNewSortOrder::Random => anjuman_contracts::deck_options::NewSortOrder::Random,
        }
    }
}

impl DbNewSortOrder {
    /// The order as a lowercase string, for binding enum columns via a
    /// `::new_sort_order` SQL cast in `query!` macros.
    pub fn as_str(self) -> &'static str {
        match self {
            DbNewSortOrder::CardTypeThenGathered => "card_type_then_gathered",
            DbNewSortOrder::Gathered => "gathered",
            DbNewSortOrder::CardTypeThenRandom => "card_type_then_random",
            DbNewSortOrder::RandomNoteThenCardType => "random_note_then_card_type",
            DbNewSortOrder::Random => "random",
        }
    }
}

impl DbCardState {
    /// The card state as a lowercase string (e.g. `"learning"`), matching the
    /// application's existing `state: String` wire fields.
    pub fn as_str(self) -> &'static str {
        match self {
            DbCardState::New => "new",
            DbCardState::Learning => "learning",
            DbCardState::Review => "review",
            DbCardState::Relearning => "relearning",
        }
    }
}
