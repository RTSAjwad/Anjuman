//! Core application logic (Elm-style `Model` / `Event` / `ViewModel` / `Effect`).
//!
//! This module has **no knowledge of any UI**. It only declares what changes and
//! what side effects to perform; each shell decides *how* to render and execute.

use crux_core::{
    Command,
    macros::effect,
    render::{self, RenderOperation},
};
use crux_http::{command::Http, protocol::HttpRequest};
use std::collections::HashMap;

use crux_kv::protocol::KeyValueOperation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use anjuman_contracts::auth::{LoginRequest, LoginResponse, Permissions, UserResponse};
use anjuman_contracts::cards::{
    CardBrowserPage, CardBrowserQuery, CardBrowserResponse, CardModResponse, NoteModResponse,
    RescheduleBody,
};
use anjuman_contracts::decks::{DeckCountsResponse, DeckResponse, SetDeckOptions};
use anjuman_contracts::deck_options::{DeckOptions, UpdateDeckOptions};
use anjuman_contracts::note_types::NoteTypeResponse;
use anjuman_contracts::notes::{
    CreateNote, ListNotesQuery, NoteBrowserPage, NoteResponse, UpdateNote,
};
use anjuman_contracts::reviews::{FlagResponse, SetFlag};
use anjuman_contracts::MessageResponse;
use anjuman_contracts::study::{StudyAdvance, StudyAdvanceBody, StudyCard, StudyCounts};
use anjuman_contracts::tags::{Tag, TagNode};

/// The base URL of the Anjuman server.
///
/// Uses `127.0.0.1` rather than `localhost` to avoid the browser resolving
/// `localhost` to IPv6 `::1` first (the server binds IPv4 `127.0.0.1` only),
/// which would fail the connection even after CORS is configured.
const API_URL: &str = "http://127.0.0.1:3000";

/// The key under which the JWT is persisted (by the shell's KV capability).
const TOKEN_KEY: &str = "auth_token";

/// Which sub-view of the selected deck is showing. This is the core-owned
/// source of truth for the deck "route" (the shell renders it, it doesn't hold
/// its own). `Study` is the default when re-opening a deck.
#[derive(Serialize, Deserialize, Facet, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum DeckView {
    #[default]
    Study,
    Options,
}

/// The single top-level route the user is on. This is the **core-owned** source
/// of truth for navigation — every shell renders `ViewModel.route`, and never
/// holds its own routing state.
///
/// The flat variants cover every top-level destination:
/// - `DeckList`    — the deck list (default when authenticated, nothing selected).
/// - `Cards`       — the card browser (US-4.12).
/// - `Notes`       — the notes browser (US-4.13).
/// - `Study`       — a deck, on its Study panel.
/// - `DeckOptions` — a deck, on its Options panel.
#[derive(Serialize, Deserialize, Facet, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CurrentRoute {
    #[default]
    DeckList,
    Cards,
    Notes,
    Study,
    DeckOptions,
}

/// The entire application state.
#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    /// The authenticated session, if any.
    pub auth: Auth,
    /// True while a login/restore round-trip is in flight (to show a spinner).
    pub busy: bool,
    /// A human-readable error from the last auth attempt, if any.
    pub error: Option<String>,
    /// Transient token carried from `RestoreSession` → `/me` → `MeResult` (the
    /// token is known at restore time but the user comes back from `/me`).
    pub pending_token: Option<String>,
    /// The signed-in user's decks (from `GET /decks`).
    pub decks: Vec<DeckResponse>,
    /// A human-readable error from the last decks fetch, if any.
    pub decks_error: Option<String>,
    /// The deck the user has opened to study (null until one is selected).
    pub selected_deck: Option<DeckSummary>,
    /// Which sub-view of the selected deck is showing (Study or Options).
    pub selected_deck_view: DeckView,
    /// The top-level route the user is on (the single navigation source of
    /// truth). Derived in `view()` from selection + this explicit field.
    pub route: CurrentRoute,
    /// The card currently shown, if study has started and a card is due.
    pub current_card: Option<StudyCard>,
    /// Due counts for the deck being studied.
    pub counts: StudyCounts,
    /// A human-readable error from the last study request, if any.
    pub study_error: Option<String>,
    /// The selected deck's scheduling preset (from `GET /deck-options/{id}`),
    /// once fetched.
    pub deck_options: Option<DeckOptions>,
    /// A human-readable error from the last deck-options fetch, if any.
    pub deck_options_error: Option<String>,
    /// The school's reusable presets (from `GET /deck-options`), for the
    /// US-4.11 assign-preset affordance.
    pub presets: Vec<DeckOptions>,
    /// A human-readable error from the last preset-list fetch, if any.
    pub presets_error: Option<String>,
    /// The distinct tag namespace (from `GET /tags`), for the sidebar.
    pub tags: Vec<Tag>,
    /// The distinct tag namespace as a `::` tree (from `GET /tags/tree`).
    pub tag_tree: Vec<TagNode>,
    /// A human-readable error from the last tag-list fetch, if any.
    pub tags_error: Option<String>,
    /// A human-readable error from the last card action (bury/suspend/
    /// reschedule/flag) during study, if any.
    pub card_action_error: Option<String>,
    /// The card-browser result set + page info (from `GET /cards`).
    pub browser_cards: Vec<CardBrowserResponse>,
    pub browser_page: i64,
    pub browser_per_page: i64,
    pub browser_total: i64,
    /// The card currently selected in the browser (for per-card actions).
    pub browser_selected: Option<CardBrowserResponse>,
    /// The last filter used to fetch the browser (so actions can refresh).
    pub browser_filter: Option<CardBrowserFilter>,
    /// A human-readable error from the last card-browser fetch, if any.
    pub browser_error: Option<String>,
    /// The school's note types (from `GET /note-types`), for the browser's
    /// note-type filter dropdown (US-4.12 carve-out).
    pub note_types: Vec<NoteTypeResponse>,
    /// A human-readable error from the last note-types fetch, if any.
    pub note_types_error: Option<String>,
    /// The notes browser result set (from `GET /notes`).
    pub notes: Vec<NoteResponse>,
    /// Notes page info (page/per-page/total).
    pub notes_page: i64,
    pub notes_per_page: i64,
    pub notes_total: i64,
    /// The note currently selected in the browser (for editor/actions).
    pub notes_selected: Option<NoteResponse>,
    /// The last notes filter used, so create/save/delete can refresh.
    pub notes_filter: Option<NoteBrowserFilter>,
    /// A human-readable error from the last notes fetch, if any.
    pub notes_error: Option<String>,
}

/// Authentication state.
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub enum Auth {
    #[default]
    Unauthenticated,
    Authenticated {
        token: String,
        user: UserResponse,
    },
}

/// Actions the user (or a running effect) can trigger.
///
/// The `#[serde(skip)]`/`#[facet(skip)]` events are *core-local*: they carry
/// deserialized capability responses and never cross the FFI boundary (the shell
/// forwards raw bytes via `resolve`, and the core constructs these itself).
#[derive(Serialize, Deserialize, Facet, Debug, Clone)]
#[repr(C)]
pub enum Event {
    /// Submit the login form.
    LoginSubmit {
        email: String,
        password: String,
    },
    /// Attempt to restore a persisted session on startup.
    RestoreSession,
    /// Sign out (clear the stored token + drop session state).
    Logout,
    /// Fetch the signed-in user's decks (fired after a successful login/restore).
    DecksRequested,
    /// Open a deck (by id) to study it — records the selection and switches the
    /// shell from the list view to the study screen.
    OpenDeck {
        deck_id: i64,
    },
    /// Open a deck directly into its Options sub-view (deck-options screen).
    OpenDeckOptions {
        deck_id: i64,
    },
    /// Begin studying the selected deck: fetch the first due card (+ counts).
    StartStudy,
    /// Fetch the selected deck's scheduling preset (its deck options), resolved
    /// via the deck's effective `options_id`.
    DeckOptionsRequested {
        deck_id: i64,
    },
    /// Save edits to the selected deck's preset (only `Some` fields are changed).
    DeckOptionsSave(DeckOptionsEdit),
    /// Assign the selected deck's scheduling preset (US-4.11). `options_id: 0`
    /// means "use the default".
    DeckOptionsAssign {
        deck_id: i64,
        options_id: i64,
    },
    /// Fetch the school's reusable presets (for the assign-preset affordance).
    DeckOptionsListRequested,
    /// Fetch the distinct tag namespace (for the sidebar).
    TagsRequested,
    /// Fetch the distinct tag namespace as a `::` tree (for the sidebar).
    TagTreeRequested,
    /// Fetch the school's note types (for the card-browser note-type filter).
    NoteTypesRequested,
    /// Fetch notes with a filter (page/filter/sort), the note analog of
    /// `CardsRequested`.
    NotesRequested {
        filter: NoteBrowserFilter,
    },
    /// Create a note (fields keyed by note-type field name → text value).
    NoteCreate {
        note_type_id: i64,
        deck_id: i64,
        fields: HashMap<String, String>,
    },
    /// Save edits to a note (only `Some` fields change).
    NoteSave {
        note_id: i64,
        note_type_id: Option<i64>,
        fields: Option<HashMap<String, String>>,
    },
    /// Delete a note (and its cards).
    NoteDelete {
        note_id: i64,
    },
    /// Answer the current card with a rating (1-4: Again/Hard/Good/Easy).
    Answer {
        rating: i32,
    },
    /// Bury the current card until the next day-start.
    BuryCard,
    /// Bury all cards of the current card's note until the next day-start.
    BuryNote,
    /// Suspend the current card.
    SuspendCard,
    /// Unsuspend the current card.
    UnsuspendCard,
    /// Bury the current card until the next day-start.
    UnburyCard,
    /// Suspend all cards of the current card's note.
    SuspendNote,
    /// Reschedule the current card to `days` days from now (Anki "set due date").
    RescheduleCard {
        days: i64,
    },
    /// Set the current card's flag (0-7; 0 clears the flag).
    FlagCard {
        flag: i32,
    },
    /// Fetch the card browser (US-4.12) with the given filter.
    CardsRequested {
        filter: CardBrowserFilter,
    },
    /// Select a card in the browser (enables per-card actions).
    BrowserSelectCard {
        card_id: i64,
    },
    /// Select a note in the browser (enables the editor/actions).
    NoteBrowserSelectNote {
        note_id: i64,
    },
    /// Suspend a browser card by id.
    BrowserSuspendCard {
        card_id: i64,
    },
    /// Unsuspend a browser card by id.
    BrowserUnsuspendCard {
        card_id: i64,
    },
    /// Bury a browser card by id.
    BrowserBuryCard {
        card_id: i64,
    },
    /// Unbury a browser card by id.
    BrowserUnburyCard {
        card_id: i64,
    },
    /// Reschedule a browser card by id to `days` from now.
    BrowserRescheduleCard {
        card_id: i64,
        days: i64,
    },
    /// Set a browser card's flag by id (0-7).
    BrowserFlagCard {
        card_id: i64,
        flag: i32,
    },
    /// Leave the study screen and return to the deck list: clear the selection
    /// and reset in-flight study state (keeps auth + deck data intact).
    CloseDeck,
    /// Navigate to the deck list (top-level nav — clears any open deck).
    ShowDeckList,
    /// Navigate to the card browser (top-level nav — clears any open deck).
    ShowCards,
    /// Navigate to the notes browser (top-level nav — clears any open deck).
    ShowNotes,

    // --- Core-local completion events (never cross the FFI boundary) ---

    #[serde(skip)]
    #[facet(skip)]
    StudyStarted(#[facet(opaque)] crux_http::Result<crux_http::Response<StudyAdvance>>),

    #[serde(skip)]
    #[facet(skip)]
    DeckOptionsResult(#[facet(opaque)] crux_http::Result<crux_http::Response<DeckOptions>>),

    #[serde(skip)]
    #[facet(skip)]
    DeckOptionsSaved(#[facet(opaque)] crux_http::Result<crux_http::Response<DeckOptions>>),

    #[serde(skip)]
    #[facet(skip)]
    DeckOptionsListResult(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<DeckOptions>>>),

    #[serde(skip)]
    #[facet(skip)]
    DeckOptionsAssigned(#[facet(opaque)] crux_http::Result<crux_http::Response<DeckResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    StudyAdvanced(#[facet(opaque)] crux_http::Result<crux_http::Response<StudyAdvance>>),

    #[serde(skip)]
    #[facet(skip)]
    DecksResult(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<DeckResponse>>>),

    #[serde(skip)]
    #[facet(skip)]
    DecksCountsResult(#[facet(opaque)] crux_http::Result<crux_http::Response<DeckCountsResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    LoginResult(#[facet(opaque)] crux_http::Result<crux_http::Response<LoginResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    MeResult(#[facet(opaque)] crux_http::Result<crux_http::Response<UserResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    TokenRead(#[facet(opaque)] crux_kv::DataResult),

    #[serde(skip)]
    #[facet(skip)]
    TokenStored(#[facet(opaque)] crux_kv::DataResult),

    #[serde(skip)]
    #[facet(skip)]
    TokenDeleted(#[facet(opaque)] crux_kv::DataResult),

    #[serde(skip)]
    #[facet(skip)]
    TagsResult(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<Tag>>>),

    #[serde(skip)]
    #[facet(skip)]
    TagTreeResult(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<TagNode>>>),

    #[serde(skip)]
    #[facet(skip)]
    CardActionDone(#[facet(opaque)] crux_http::Result<crux_http::Response<CardModResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    NoteActionDone(#[facet(opaque)] crux_http::Result<crux_http::Response<NoteModResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    RescheduleDone(#[facet(opaque)] crux_http::Result<crux_http::Response<CardModResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    FlagDone(#[facet(opaque)] crux_http::Result<crux_http::Response<FlagResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    NoteTypesResult(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<NoteTypeResponse>>>),

    #[serde(skip)]
    #[facet(skip)]
    NotesResult(#[facet(opaque)] crux_http::Result<crux_http::Response<NoteBrowserPage>>),

    #[serde(skip)]
    #[facet(skip)]
    NoteCreated(#[facet(opaque)] crux_http::Result<crux_http::Response<NoteResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    NoteSaved(#[facet(opaque)] crux_http::Result<crux_http::Response<NoteResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    NoteDeleted(#[facet(opaque)] crux_http::Result<crux_http::Response<MessageResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    CardsResult(#[facet(opaque)] crux_http::Result<crux_http::Response<CardBrowserPage>>),

    #[serde(skip)]
    #[facet(skip)]
    BrowserCardActionDone(
        #[facet(opaque)] crux_http::Result<crux_http::Response<CardModResponse>>,
    ),

    #[serde(skip)]
    #[facet(skip)]
    BrowserRescheduleDone(
        #[facet(opaque)] crux_http::Result<crux_http::Response<CardModResponse>>,
    ),

    #[serde(skip)]
    #[facet(skip)]
    BrowserFlagDone(#[facet(opaque)] crux_http::Result<crux_http::Response<FlagResponse>>),
}

/// Side effects the core requests from the shell.
///
/// `Http` carries an `HttpRequest` (performed by the shell via `crux_http`);
/// `KeyValue` carries a `KeyValueOperation` (persisted by the shell via
/// `crux_kv`); `Render` asks for a UI refresh.
///
/// NOTE: `#[effect(facet_typegen)]` generates the necessary traits (including
/// serialization and `Facet`) for the FFI enum itself — it must NOT be combined
/// with a manual `#[derive(Serialize, Deserialize)]`.
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
}

/// The precise state the UI needs to render.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Debug)]
pub struct ViewModel {
    pub email: String,
    pub authenticated: bool,
    /// The signed-in user's display name, when authenticated.
    pub display_name: String,
    /// The user's global capabilities (server-resolved role bundle); the shell
    /// gates management affordances on these. Study is gated per-deck via
    /// `DeckSummary.studyable`, not here.
    pub permissions: PermissionsView,
    pub busy: bool,
    pub error: Option<String>,
    /// Decks to render (title + per-state counts).
    pub decks: Vec<DeckSummary>,
    /// The selected deck (id + title + studyable) to show on the study screen,
    /// or `None` when showing the list.
    pub selected_deck: Option<DeckSummary>,
    /// Which sub-view of the selected deck is showing (Study or Options).
    pub selected_deck_view: DeckView,
    /// The top-level route to render (`DeckList` | `Cards` | `Notes` | `Study` |
    /// `DeckOptions`). The shell renders this; it never holds routing state.
    pub route: CurrentRoute,
    /// A human-readable error from the last decks fetch, if any.
    pub decks_error: Option<String>,
    /// The card currently shown during study, or `None` when nothing is due
    /// (study not started, or the deck is finished).
    pub current_card: Option<StudyCardView>,
    /// Due counts for the deck being studied.
    pub counts: StudyCountsView,
    /// A human-readable error from the last study request, if any.
    pub study_error: Option<String>,
    /// The selected deck's scheduling preset, once fetched (US-4.9).
    pub deck_options: Option<DeckOptionsView>,
    /// A human-readable error from the last deck-options fetch, if any.
    pub deck_options_error: Option<String>,
    /// The school's reusable presets (id + name) for the assign-preset
    /// affordance (US-4.11).
    pub presets: Vec<PresetSummary>,
    /// A human-readable error from the last preset-list fetch, if any.
    pub presets_error: Option<String>,
    /// The distinct tag namespace (for the sidebar).
    pub tags: Vec<TagView>,
    /// The distinct tag namespace as a `::` tree (for the sidebar).
    pub tag_tree: Vec<TagNodeView>,
    /// A human-readable error from the last tag-list fetch, if any.
    pub tags_error: Option<String>,
    /// A human-readable error from the last card action (bury/suspend/
    /// reschedule/flag) during study, if any.
    pub card_action_error: Option<String>,
    /// The card-browser results to render (US-4.12).
    pub browser_cards: Vec<CardBrowserView>,
    pub browser_page: i64,
    pub browser_per_page: i64,
    pub browser_total: i64,
    /// The card currently selected in the browser, for per-card actions.
    pub browser_selected: Option<CardBrowserView>,
    /// A human-readable error from the last card-browser fetch, if any.
    pub browser_error: Option<String>,
    /// The school's note types (id + name) for the browser's filter dropdown.
    pub note_types: Vec<NoteTypeView>,
    /// A human-readable error from the last note-types fetch, if any.
    pub note_types_error: Option<String>,
    /// The notes browser results to render (US-4.13).
    pub notes: Vec<NoteView>,
    /// Notes page info.
    pub notes_page: i64,
    pub notes_per_page: i64,
    pub notes_total: i64,
    /// The note currently selected in the browser, for the editor/actions.
    pub notes_selected: Option<NoteView>,
    /// A human-readable error from the last notes fetch/action, if any.
    pub notes_error: Option<String>,
}

/// The shell-facing view of the user's global capabilities (a Facet mirror of
/// the wire `Permissions`). Study is *not* here — it is per-deck
/// (`DeckSummary.studyable`); these are the management/role-bundle capabilities.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct PermissionsView {
    pub can_administer_school: bool,
    pub can_manage_users: bool,
    pub can_manage_decks: bool,
    pub can_manage_notes: bool,
    pub can_manage_presets: bool,
    pub can_manage_note_types: bool,
    pub can_manage_tags: bool,
    pub can_manage_classes: bool,
    pub can_view_dashboard: bool,
    pub can_view_analytics: bool,
}

impl From<&Permissions> for PermissionsView {
    fn from(p: &Permissions) -> Self {
        PermissionsView {
            can_administer_school: p.can_administer_school,
            can_manage_users: p.can_manage_users,
            can_manage_decks: p.can_manage_decks,
            can_manage_notes: p.can_manage_notes,
            can_manage_presets: p.can_manage_presets,
            can_manage_note_types: p.can_manage_note_types,
            can_manage_tags: p.can_manage_tags,
            can_manage_classes: p.can_manage_classes,
            can_view_dashboard: p.can_view_dashboard,
            can_view_analytics: p.can_view_analytics,
        }
    }
}

/// The shell-facing card-browser filter (US-4.12). The core folds these into the
/// wire `CardBrowserQuery`'s comma-separated strings when building the request.
/// Empty `Vec`s mean "no filter"; `None` means "not set".
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct CardBrowserFilter {
    pub deck_ids: Vec<i64>,
    pub note_type_ids: Vec<i64>,
    pub q: Option<String>,
    /// `new`, `learning`, `review`, `relearning`.
    pub states: Vec<String>,
    /// Anki-style flag values 0-7 (empty = no filter).
    pub flags: Vec<i64>,
    pub sort: String,
    pub page: i64,
    pub per_page: i64,
}

impl CardBrowserFilter {
    /// Fold into the wire `CardBrowserQuery` (comma-separated multi-values),
    /// defaulting `sort`/`page`/`per_page` when unset.
    pub fn to_wire_query(&self) -> CardBrowserQuery {
        CardBrowserQuery {
            deck_id: csv_i64(&self.deck_ids),
            note_type_id: csv_i64(&self.note_type_ids),
            q: self.q.clone(),
            state: self.states.join(","),
            flag: csv_i64(&self.flags),
            sort: if self.sort.is_empty() {
                "created_at".to_string()
            } else {
                self.sort.clone()
            },
            page: if self.page > 0 { self.page } else { 1 },
            per_page: if self.per_page > 0 { self.per_page } else { 50 },
        }
    }
}

/// The shell-facing note-browser filter (US-4.13, widened for the merged
/// browser). Mirrors `CardBrowserFilter` but carries only note-appropriate
/// filters (`deck_ids`, `note_type_ids`, `q`, `sort`, `page`, `per_page`).
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct NoteBrowserFilter {
    pub deck_ids: Vec<i64>,
    pub note_type_ids: Vec<i64>,
    pub q: Option<String>,
    pub sort: String,
    pub page: i64,
    pub per_page: i64,
}

impl NoteBrowserFilter {
    /// Fold into the wire `ListNotesQuery` (comma-separated multi-values),
    /// defaulting `sort`/`page`/`per_page` when unset.
    pub fn to_wire_query(&self) -> ListNotesQuery {
        ListNotesQuery {
            deck_id: csv_i64(&self.deck_ids),
            note_type_id: csv_i64(&self.note_type_ids),
            q: self.q.clone(),
            sort: if self.sort.is_empty() {
                "created_at".to_string()
            } else {
                self.sort.clone()
            },
            page: if self.page > 0 { self.page } else { 1 },
            per_page: if self.per_page > 0 { self.per_page } else { 50 },
        }
    }
}

/// Join `i64`s into a comma-separated string (empty for an empty list).
fn csv_i64(vals: &[i64]) -> String {
    vals.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Clear the deck selection + any in-flight study/options state (shared by
/// `CloseDeck` and the top-level `Show*` navigation events). Does not touch
/// auth, decks, tags, or the card/notes browser data.
fn clear_deck_selection(model: &mut Model) {
    model.selected_deck = None;
    model.selected_deck_view = DeckView::Study;
    model.current_card = None;
    model.counts = StudyCounts::default();
    model.study_error = None;
    model.deck_options = None;
    model.deck_options_error = None;
    model.card_action_error = None;
}

/// A shell-facing browser card (a Facet mirror of the wire `CardBrowserResponse`).
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Debug)]
pub struct CardBrowserView {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub deck_title: String,
    pub note_type_name: String,
    pub front: String,
    pub back: String,
    pub state: String,
    pub flag: i64,
    pub suspended: bool,
    pub buried: bool,
}

impl From<&CardBrowserResponse> for CardBrowserView {
    fn from(c: &CardBrowserResponse) -> Self {
        CardBrowserView {
            card_id: c.card_id,
            note_id: c.note_id,
            deck_id: c.deck_id,
            deck_title: c.deck_title.clone(),
            note_type_name: c.note_type_name.clone(),
            front: c.front.clone(),
            back: c.back.clone(),
            state: c.state.clone().unwrap_or_default(),
            flag: c.flag.unwrap_or(0),
            suspended: c.suspended.unwrap_or(false),
            buried: c.buried_at.is_some(),
        }
    }
}

/// A shell-facing note type for pickers (a Facet mirror of the wire
/// `NoteTypeResponse`, carrying the fields a picker + editor needs).
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct NoteTypeView {
    pub id: i64,
    pub name: String,
    /// The note type's field names, in order (drives the note editor inputs).
    pub field_names: Vec<String>,
}

impl From<&NoteTypeResponse> for NoteTypeView {
    fn from(n: &NoteTypeResponse) -> Self {
        NoteTypeView {
            id: n.id,
            name: n.name.clone(),
            field_names: n.field_names.clone(),
        }
    }
}

/// A shell-facing note (a Facet mirror of the wire `NoteResponse`). Fields are
/// flattened to string values (the editor's text inputs); tags to names.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct NoteView {
    pub id: i64,
    pub note_type_id: i64,
    pub note_type_name: String,
    pub fields: HashMap<String, String>,
    pub tags: Vec<String>,
    /// The resolved sort-field value (see `NoteResponse.sort_field`).
    pub sort_field: Option<String>,
}

impl From<&NoteResponse> for NoteView {
    fn from(n: &NoteResponse) -> Self {
        let fields = n
            .fields
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), s)
            })
            .collect();
        let tags = n.tags.iter().map(|t| t.name.clone()).collect();
        NoteView {
            id: n.id,
            note_type_id: n.note_type_id,
            note_type_name: n.note_type_name.clone(),
            fields,
            tags,
            sort_field: n.sort_field.clone(),
        }
    }
}

/// The shell-facing view of a card during study (a Facet/FFI-friendly mirror of
/// the wire `StudyCard`, following the same core-local mapping as `DeckSummary`
/// vs. `DeckResponse`). Bury/suspend and flags are out of scope for now.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct StudyCardView {
    pub card_id: i64,
    /// The note this card was generated from (for note-level bury/suspend).
    pub note_id: i64,
    pub front: String,
    pub back: String,
    /// The scheduling state label (`new`, `learning`, `review`, `relearning`).
    pub state: String,
    /// Current position in the learning/relearning steps list (0-based).
    pub step_index: i64,
    /// Predicted interval (in seconds) until next review per rating, keyed by
    /// rating 1..=4 (1=Again, 2=Hard, 3=Good, 4=Easy). `None` when the server
    /// returned no prediction (e.g. certain new/learning states).
    pub predicted_interval: Option<HashMap<i32, i64>>,
    /// The note type's shared styling CSS; injected into the card so it renders
    /// self-contained (empty string when the note type has no styling).
    pub styling: String,
    /// Anki-style card flag (0-7; 0 = no flag).
    pub flag: i64,
    /// True when the card is currently suspended.
    pub suspended: bool,
    /// True when the card is currently buried (`buried_at` set).
    pub buried: bool,
}

/// The shell-facing per-state counts for a study session (a Facet-friendly
/// mirror of the wire `StudyCounts`).
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct StudyCountsView {
    pub new_count: i64,
    pub learning_count: i64,
    pub review_count: i64,
    pub relearning_count: i64,
}

impl From<StudyCounts> for StudyCountsView {
    fn from(c: StudyCounts) -> Self {
        StudyCountsView {
            new_count: c.new_count,
            learning_count: c.learning_count,
            review_count: c.review_count,
            relearning_count: c.relearning_count,
        }
    }
}

impl From<&StudyCard> for StudyCardView {
    fn from(c: &StudyCard) -> Self {
        StudyCardView {
            card_id: c.card_id,
            note_id: c.note_id,
            front: c.front.clone(),
            back: c.back.clone(),
            state: c.state.clone(),
            step_index: c.step_index,
            predicted_interval: c.predicted_interval.as_ref().map(|m| {
                m.iter()
                    .filter_map(|(k, v)| k.parse::<i32>().ok().map(|rating| (rating, *v)))
                    .collect()
            }),
            styling: c.styling.clone(),
            flag: c.flag,
            suspended: c.suspended,
            buried: c.buried_at.is_some(),
        }
    }
}

/// A deck as shown in the list view (title + due counts), with subdecks nested
/// under their parent. The **core** builds this tree (the shell only renders
/// `children` recursively); orphans (`parent_id` pointing at a missing/absent
/// deck) are surfaced at the root in input order.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct DeckSummary {
    pub id: i64,
    pub title: String,
    pub new_count: i64,
    pub learning_count: i64,
    pub review_count: i64,
    pub total_count: i64,
    /// Whether the user may study this deck (US-2.19: a context-only ancestor
    /// is `false`). The shell disables the study affordance for `false` decks.
    pub studyable: bool,
    /// Subdecks of this deck, in input order.
    pub children: Vec<DeckSummary>,
}

/// Build the nested deck tree from the flat `GET /decks` response.
///
/// Children are attached to their parent by `parent_id` (decks whose
/// `parent_id` is `None` or points at a deck not in the list become roots), and
/// both roots and siblings keep the order of the input list.
fn build_deck_tree(decks: &[DeckResponse]) -> Vec<DeckSummary> {
    // Phase 1: one `DeckSummary` per deck (children empty initially), keyed by
    // id. Keep them in a map so we can attach children without moving parents
    // out from under their own children.
    let mut by_id: std::collections::HashMap<i64, DeckSummary> = decks
        .iter()
        .map(|d| {
            (
                d.id,
                DeckSummary {
                    id: d.id,
                    title: d.title.clone(),
                    new_count: d.new_count.unwrap_or(0),
                    learning_count: d.learning_count.unwrap_or(0),
                    review_count: d.review_count.unwrap_or(0),
                    total_count: d.total_count.unwrap_or(0),
                    studyable: d.studyable,
                    children: Vec::new(),
                },
            )
        })
        .collect();

    // Phase 2: for each input deck (in order), move it into its parent's
    // `children` (or keep it as a root). We take-and-reinsert so a deck can be
    // both a child and a parent of deeper decks.
    let mut roots: Vec<i64> = Vec::new();
    for d in decks {
        match d.parent_id {
            Some(pid) if by_id.contains_key(&pid) => {
                if let Some(node) = by_id.remove(&d.id) {
                    by_id.get_mut(&pid).unwrap().children.push(node);
                }
            }
            _ => roots.push(d.id),
        }
    }

    roots
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

/// Find a deck (by id) in the flat `GET /decks` list, returning a `DeckSummary`
/// clone for selection. Returns `None` for an unknown id.
fn find_deck(decks: &[DeckResponse], deck_id: i64) -> Option<DeckSummary> {
    let d = decks.iter().find(|d| d.id == deck_id)?;
    Some(DeckSummary {
        id: d.id,
        title: d.title.clone(),
        new_count: d.new_count.unwrap_or(0),
        learning_count: d.learning_count.unwrap_or(0),
        review_count: d.review_count.unwrap_or(0),
        total_count: d.total_count.unwrap_or(0),
        studyable: d.studyable,
        children: Vec::new(),
    })
}

/// Fold the latest study-screen counts back into the selected deck's entry in
/// the flat deck list, so the deck-list badges and the study-screen badges are
/// one source of truth (write-through — no extra `/decks/counts` fetch).
///
/// Maps `StudyCounts`' `new/learning/review/relearning` into the matching
/// `DeckResponse` `Option<i64>` fields. `total_count` is **left untouched**: it
/// is a card count (not a due count), and `StudyCounts` does not carry it, so a
/// completed card must not reduce it.
fn fold_study_counts_into_decks(
    decks: &mut [DeckResponse],
    selected_deck: Option<&DeckSummary>,
    counts: StudyCounts,
) {
    let Some(selected) = selected_deck else {
        return;
    };
    let Some(deck) = decks.iter_mut().find(|d| d.id == selected.id) else {
        return;
    };
    deck.new_count = Some(counts.new_count);
    deck.learning_count = Some(counts.learning_count);
    deck.review_count = Some(counts.review_count);
    deck.relearning_count = Some(counts.relearning_count);
}

/// Re-fetch the card browser using the last stored filter (returns a `Render` for
/// the current state, chained with a `CardsRequested` command). For browser action
/// completions, which refresh the list rather than the study loop.
fn refresh_browser(model: &Model) -> Command<Effect, Event> {
    let Some(filter) = model.browser_filter.clone() else {
        return render::render();
    };
    render::render().and(Command::event(Event::CardsRequested { filter }))
}

/// Re-sync `browser_selected` to the matching card in `browser_cards` (the single
/// source of truth), so the shell's inspector reflects the freshest data after a
/// list refresh or an in-place flag update. Clears to `None` when the selected
/// card is no longer in the list.
fn resync_browser_selected(model: &mut Model) {
    let Some(sel_id) = model.browser_selected.as_ref().map(|c| c.card_id) else {
        return;
    };
    model.browser_selected = model.browser_cards.iter().find(|c| c.card_id == sel_id).cloned();
}

/// Re-sync `notes_selected` to the matching note in `model.notes` (the single
/// source of truth) after a list refresh or edit. Clears to `None` when the
/// selected note is no longer present.
fn resync_notes_selected(model: &mut Model) {
    let Some(sel_id) = model.notes_selected.as_ref().map(|n| n.id) else {
        return;
    };
    model.notes_selected = model.notes.iter().find(|n| n.id == sel_id).cloned();
}

/// Re-fetch the notes list using the last stored filter.
fn refresh_notes(model: &Model) -> Command<Effect, Event> {
    let Some(filter) = model.notes_filter.clone() else {
        // No filter recorded yet — fall back to a default first page.
        return render::render().and(Command::event(Event::NotesRequested {
            filter: NoteBrowserFilter::default(),
        }));
    };
    render::render().and(Command::event(Event::NotesRequested { filter }))
}

/// Convert the editor's `String`-valued fields into the wire `Map<String, Value>`.
fn fields_to_map(fields: HashMap<String, String>) -> serde_json::Map<String, serde_json::Value> {
    fields
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect()
}

/// The shell-facing view of a deck-options preset (a `Facet`/`PartialEq`-friendly
/// mirror of the contracts `DeckOptions`, following the same pattern as
/// `DeckSummary`/`StudyCardView`). Enums are exposed as their serde `snake_case`
/// string names so the shell can render human labels without depending on the
/// contracts enum types (which don't implement `Facet`).
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Debug)]
pub struct DeckOptionsView {
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
    pub maximum_answer_seconds: i64,
    pub maximum_interval: i64,
    pub new_gather_order: String,
    pub new_sort_order: String,
    pub new_review_order: String,
    pub interday_order: String,
    pub review_sort_order: String,
    pub insertion_order: String,
    pub leech_action: String,
    pub show_on_screen_timer: bool,
    pub stop_timer_on_answer: bool,
    pub dont_play_audio_automatically: bool,
    pub skip_question_when_replaying_answer: bool,
    pub auto_advance_seconds_show_question: f64,
    pub auto_advance_seconds_show_answer: f64,
    pub auto_advance_wait_for_audio: bool,
    pub auto_advance_question_action: String,
    pub auto_advance_answer_action: String,
    pub easy_days: Vec<String>,
    pub fsrs_parameters: Vec<f32>,
}

impl From<&DeckOptions> for DeckOptionsView {
    fn from(o: &DeckOptions) -> Self {
        DeckOptionsView {
            id: o.id,
            school_id: o.school_id,
            name: o.name.clone(),
            learning_steps: o.learning_steps.clone(),
            relearning_steps: o.relearning_steps.clone(),
            desired_retention: o.desired_retention,
            bury_new: o.bury_new,
            bury_review: o.bury_review,
            bury_interday: o.bury_interday,
            new_per_day: o.new_per_day,
            review_per_day: o.review_per_day,
            leech_threshold: o.leech_threshold,
            maximum_answer_seconds: o.maximum_answer_seconds,
            maximum_interval: o.maximum_interval,
            new_gather_order: serde_name(&o.new_gather_order),
            new_sort_order: serde_name(&o.new_sort_order),
            new_review_order: serde_name(&o.new_review_order),
            interday_order: serde_name(&o.interday_order),
            review_sort_order: serde_name(&o.review_sort_order),
            insertion_order: serde_name(&o.insertion_order),
            leech_action: serde_name(&o.leech_action),
            show_on_screen_timer: o.show_on_screen_timer,
            stop_timer_on_answer: o.stop_timer_on_answer,
            dont_play_audio_automatically: o.dont_play_audio_automatically,
            skip_question_when_replaying_answer: o.skip_question_when_replaying_answer,
            auto_advance_seconds_show_question: o.auto_advance_seconds_show_question,
            auto_advance_seconds_show_answer: o.auto_advance_seconds_show_answer,
            auto_advance_wait_for_audio: o.auto_advance_wait_for_audio,
            auto_advance_question_action: serde_name(&o.auto_advance_question_action),
            auto_advance_answer_action: serde_name(&o.auto_advance_answer_action),
            easy_days: o.easy_days.iter().map(serde_name).collect(),
            fsrs_parameters: o.fsrs_parameters.clone(),
        }
    }
}

/// A shell-facing summary of a reusable preset (id + name) for the US-4.11
/// assign-preset affordance. Mirrors `DeckSummary`; `DeckOptions` itself isn't
/// `Facet`-derived, so this carries just the fields the picker needs.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct PresetSummary {
    pub id: i64,
    pub name: String,
}

impl From<&DeckOptions> for PresetSummary {
    fn from(o: &DeckOptions) -> Self {
        PresetSummary {
            id: o.id,
            name: o.name.clone(),
        }
    }
}

/// A shell-facing tag (id + full `::`-hierarchical name) for the sidebar.
/// Mirrors `DeckSummary`; the contracts `Tag` isn't `Facet`-derived, so this
/// carries the fields the shell renders.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct TagView {
    pub id: i64,
    pub name: String,
}

impl From<&Tag> for TagView {
    fn from(t: &Tag) -> Self {
        TagView {
            id: t.id,
            name: t.name.clone(),
        }
    }
}

/// A shell-facing node in the `::` tag tree. Mirrors the contracts `TagNode`
/// (which isn't `Facet`-derived); `children` are nested recursively.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct TagNodeView {
    pub name: String,
    pub children: Vec<TagNodeView>,
}

impl From<&TagNode> for TagNodeView {
    fn from(n: &TagNode) -> Self {
        TagNodeView {
            name: n.name.clone(),
            children: n.children.iter().map(TagNodeView::from).collect(),
        }
    }
}

/// Serialize a `Serialize` value to its snake_case wire name (e.g. an enum
/// variant) and return it sans quotes.
fn serde_name<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v)
        .expect("serialize enum")
        .trim_matches('"')
        .to_string()
}

/// The client-side form of a deck-options edit (the shell forwards this; every
/// field is `Option` so only the *changed* fields are present). Enum fields are
/// `String` in their serde `snake_case` wire form (parsed back to contracts
/// enums in `to_update`). This mirrors `DeckOptionsView` and exists because the
/// contracts `UpdateDeckOptions`/enums don't implement `Facet`.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Debug)]
pub struct DeckOptionsEdit {
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
    pub maximum_answer_seconds: Option<i64>,
    pub maximum_interval: Option<i64>,
    pub leech_action: Option<String>,
    pub new_gather_order: Option<String>,
    pub new_sort_order: Option<String>,
    pub new_review_order: Option<String>,
    pub interday_order: Option<String>,
    pub review_sort_order: Option<String>,
    pub insertion_order: Option<String>,
    pub show_on_screen_timer: Option<bool>,
    pub stop_timer_on_answer: Option<bool>,
    pub dont_play_audio_automatically: Option<bool>,
    pub skip_question_when_replaying_answer: Option<bool>,
    pub auto_advance_seconds_show_question: Option<f64>,
    pub auto_advance_seconds_show_answer: Option<f64>,
    pub auto_advance_wait_for_audio: Option<bool>,
    pub auto_advance_question_action: Option<String>,
    pub auto_advance_answer_action: Option<String>,
    pub easy_days: Option<Vec<String>>,
    pub fsrs_parameters: Option<Vec<f32>>,
}

impl DeckOptionsEdit {
    /// Validate the edit against the matrix's min/max (the server rejects these;
    /// the client pre-checks so the user sees a local message instead of a failed
    /// save). Returns `Err(description)` on the first out-of-range value.
    pub fn validate(&self) -> Result<(), String> {
        checks(self.new_per_day, 0, 9999, "new cards/day")?;
        checks(self.review_per_day, 0, 9999, "max reviews/day")?;
        checks(self.leech_threshold, 1, 9999, "leech threshold")?;
        checks(self.maximum_answer_seconds, 1, 7200, "maximum answer seconds")?;
        checks(self.maximum_interval, 1, 36500, "maximum interval")?;
        if let Some(r) = self.desired_retention {
            if !(0.70..=0.99).contains(&r) {
                return Err("desired retention must be 70%–99%".into());
            }
        }
        checks_f64(self.auto_advance_seconds_show_question, 0.0, 9999.0, "auto-advance seconds (question)")?;
        checks_f64(self.auto_advance_seconds_show_answer, 0.0, 9999.0, "auto-advance seconds (answer)")?;
        if let Some(days) = &self.easy_days {
            if days.len() != 7 {
                return Err("easy days must have exactly 7 entries".into());
            }
        }
        Ok(())
    }

    /// Convert this edit into a `UpdateDeckOptions` (only the `Some` fields are
    /// set). Enum strings are parsed back to contracts enums; an unrecognised
    /// string yields `Err` (call `validate`/the caller handles it).
    pub fn to_update(&self) -> Result<UpdateDeckOptions, String> {
        Ok(UpdateDeckOptions {
            name: self.name.clone(),
            learning_steps: self.learning_steps.clone(),
            relearning_steps: self.relearning_steps.clone(),
            desired_retention: self.desired_retention,
            bury_new: self.bury_new,
            bury_review: self.bury_review,
            bury_interday: self.bury_interday,
            new_per_day: self.new_per_day,
            review_per_day: self.review_per_day,
            leech_threshold: self.leech_threshold,
            leech_action: parse_enum(self.leech_action.as_deref())?,
            new_gather_order: parse_enum(self.new_gather_order.as_deref())?,
            new_sort_order: parse_enum(self.new_sort_order.as_deref())?,
            new_review_order: parse_enum(self.new_review_order.as_deref())?,
            interday_order: parse_enum(self.interday_order.as_deref())?,
            review_sort_order: parse_enum(self.review_sort_order.as_deref())?,
            show_on_screen_timer: self.show_on_screen_timer,
            stop_timer_on_answer: self.stop_timer_on_answer,
            dont_play_audio_automatically: self.dont_play_audio_automatically,
            skip_question_when_replaying_answer: self.skip_question_when_replaying_answer,
            auto_advance_seconds_show_question: self.auto_advance_seconds_show_question,
            auto_advance_seconds_show_answer: self.auto_advance_seconds_show_answer,
            auto_advance_wait_for_audio: self.auto_advance_wait_for_audio,
            auto_advance_question_action: parse_enum(self.auto_advance_question_action.as_deref())?,
            auto_advance_answer_action: parse_enum(self.auto_advance_answer_action.as_deref())?,
            maximum_answer_seconds: self.maximum_answer_seconds,
            maximum_interval: self.maximum_interval,
            easy_days: match &self.easy_days {
                Some(days) => Some(
                    days.iter()
                        .map(|d| {
                            parse_enum(Some(d.as_str()))?
                                .ok_or_else(|| format!("unknown easy-day value: {d}"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                None => None,
            },
            insertion_order: parse_enum(self.insertion_order.as_deref())?,
            fsrs_parameters: self.fsrs_parameters.clone(),
        })
    }
}

/// Per-option range check helper (integers).
fn checks(v: Option<i64>, min: i64, max: i64, what: &str) -> Result<(), String> {
    if let Some(n) = v {
        if !(min..=max).contains(&n) {
            return Err(format!("{what} must be {min}..={max}"));
        }
    }
    Ok(())
}

/// Per-option range check helper (floats).
fn checks_f64(v: Option<f64>, min: f64, max: f64, what: &str) -> Result<(), String> {
    if let Some(n) = v {
        if !(min..=max).contains(&n) {
            return Err(format!("{what} must be {min}..={max}"));
        }
    }
    Ok(())
}

/// Parse a `None` → `None`, or a `Some(serde snake_case name)` → the matching
/// contracts enum variant (via its `Deserialize`).
fn parse_enum<T: for<'de> serde::Deserialize<'de>>(name: Option<&str>) -> Result<Option<T>, String> {
    match name {
        None => Ok(None),
        Some(s) => {
            let v = serde_json::from_str(&format!("\"{s}\""))
                .map_err(|_| format!("unknown enum value: {s}"))?;
            Ok(Some(v))
        }
    }
}

#[derive(Default)]
pub struct Anjuman;

impl crux_core::App for Anjuman {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::LoginSubmit { email, password } => {
                model.busy = true;
                model.error = None;

                let body = LoginRequest { email, password };
                let call_api = Http::post(format!("{API_URL}/auth/login"))
                    .body_json(&body)
                    .expect("serialize login request")
                    .expect_json()
                    .build()
                    .then_send(Event::LoginResult);

                render::render().and(call_api)
            }

            Event::LoginResult(Ok(mut response)) => {
                let login = response.take_body().expect("login response has a body");
                model.auth = Auth::Authenticated {
                    token: login.token.clone(),
                    user: login.user.clone(),
                };
                model.busy = false;
                model.error = None;

                // Persist the token so the session survives a reload, then fetch
                // the decks for the freshly-authenticated user.
                let store = crux_kv::KeyValue::set(TOKEN_KEY, login.token.into_bytes())
                    .then_send(Event::TokenStored);

                render::render().and(store).and(Command::event(Event::DecksRequested))
            }

            Event::LoginResult(Err(e)) => {
                model.busy = false;
                model.error = Some(e.to_string());
                render::render()
            }

            Event::RestoreSession => {
                // Read the persisted token; `TokenRead` decides whether to call
                // `/me` with it.
                crux_kv::KeyValue::get(TOKEN_KEY).then_send(Event::TokenRead)
            }

            Event::TokenRead(result) => {
                let token = match result {
                    Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
                    _ => None,
                };

                match token {
                    Some(token) => {
                        // Carry the token across the `/me` round-trip, then
                        // validate it against /me.
                        model.pending_token = Some(token.clone());
                        model.busy = true;
                        Http::get(format!("{API_URL}/me"))
                            .header("authorization", format!("Bearer {token}"))
                            .expect_json()
                            .build()
                            .then_send(Event::MeResult)
                    }
                    None => render::render(), // not signed in; nothing to do
                }
            }

            Event::MeResult(Ok(mut response)) => {
                let user = response.take_body().expect("me response has a body");
                let token = model.pending_token.take().unwrap_or_default();
                model.auth = Auth::Authenticated { token, user };
                model.busy = false;
                render::render().and(Command::event(Event::DecksRequested))
            }

            Event::MeResult(Err(_)) => {
                // Token invalid/expired: clear it and return to signed-out.
                model.pending_token = None;
                model.auth = Auth::Unauthenticated;
                model.busy = false;
                let clear = crux_kv::KeyValue::delete(TOKEN_KEY).then_send(Event::TokenDeleted);
                render::render().and(clear)
            }

            Event::Logout => {
                model.auth = Auth::Unauthenticated;
                model.busy = false;
                model.error = None;
                model.decks.clear();
                model.decks_error = None;
                model.selected_deck = None;
                model.selected_deck_view = DeckView::Study;
                model.current_card = None;
                model.counts = StudyCounts::default();
                model.study_error = None;
                model.deck_options = None;
                model.deck_options_error = None;
                model.tags.clear();
                model.tag_tree.clear();
                model.tags_error = None;
                model.card_action_error = None;
                model.browser_cards.clear();
                model.browser_selected = None;
                model.browser_filter = None;
                model.browser_error = None;
                model.note_types.clear();
                model.note_types_error = None;
                model.notes.clear();
                model.notes_filter = None;
                model.notes_error = None;
                model.notes_selected = None;
                model.route = CurrentRoute::DeckList;
                let clear = crux_kv::KeyValue::delete(TOKEN_KEY).then_send(Event::TokenDeleted);
                render::render().and(clear)
            }

            Event::OpenDeck { deck_id } => {
                // Only decks already in the list are openable; an unknown id is
                // a no-op (no crash, no bogus selection). Opening lands on the
                // Study sub-view and begins the study fetch directly.
                if let Some(summary) = find_deck(&model.decks, deck_id) {
                    model.selected_deck = Some(summary);
                    model.selected_deck_view = DeckView::Study;
                    model.route = CurrentRoute::Study;
                    return render::render().and(Command::event(Event::StartStudy));
                }
                render::render()
            }

            Event::OpenDeckOptions { deck_id } => {
                // Open a deck directly into its Options sub-view: select it and
                // fetch its preset.
                if let Some(summary) = find_deck(&model.decks, deck_id) {
                    model.selected_deck = Some(summary);
                    model.selected_deck_view = DeckView::Options;
                    model.route = CurrentRoute::DeckOptions;
                    return render::render().and(Command::event(Event::DeckOptionsRequested {
                        deck_id,
                    }));
                }
                render::render()
            }

            Event::CloseDeck => {
                // Back-navigation: drop the selection and any in-flight study
                // state, but keep auth + decks (unlike `Logout`). Re-opening a
                // deck always starts on the Study sub-view.
                clear_deck_selection(model);
                model.route = CurrentRoute::DeckList;
                render::render()
            }

            Event::ShowDeckList => {
                // Top-level nav: wins over an open deck (clears it) and lands on
                // the list.
                clear_deck_selection(model);
                model.route = CurrentRoute::DeckList;
                render::render()
            }

            Event::ShowCards => {
                // Top-level nav: leave any open deck and open the card browser.
                clear_deck_selection(model);
                model.route = CurrentRoute::Cards;
                model.browser_error = None;
                render::render()
            }

            Event::ShowNotes => {
                // Top-level nav: leave any open deck and open the notes browser.
                clear_deck_selection(model);
                model.route = CurrentRoute::Notes;
                model.notes_error = None;
                render::render()
            }

            Event::DeckOptionsRequested { deck_id } => {
                // Resolve the selected deck's effective `options_id` and fetch its
                // preset. Requires an authenticated token; otherwise a no-op.
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let Some(options_id) = model
                    .decks
                    .iter()
                    .find(|d| d.id == deck_id)
                    .map(|d| d.options_id)
                else {
                    // Unknown deck — no bogus request.
                    return render::render();
                };
                model.deck_options = None;
                model.deck_options_error = None;
                Http::get(format!("{API_URL}/deck-options/{options_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::DeckOptionsResult)
            }

            Event::DeckOptionsResult(Ok(mut response)) => {
                model.deck_options =
                    Some(response.take_body().expect("deck-options response has a body"));
                model.deck_options_error = None;
                render::render()
            }

            Event::DeckOptionsResult(Err(e)) => {
                model.deck_options = None;
                model.deck_options_error = Some(e.to_string());
                render::render()
            }

            Event::DeckOptionsSave(edit) => {
                // Requires a loaded preset (the PATCH id) + an authenticated
                // token; otherwise a no-op.
                let Some(preset) = &model.deck_options else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                // Local validation: reject out-of-range values before any
                // request is emitted, so the user sees a local error.
                if let Err(msg) = edit.validate() {
                    model.deck_options_error = Some(msg);
                    return render::render();
                }
                let update = match edit.to_update() {
                    Ok(u) => u,
                    Err(msg) => {
                        model.deck_options_error = Some(msg);
                        return render::render();
                    }
                };
                model.deck_options_error = None;
                Http::patch(format!("{API_URL}/deck-options/{}", preset.id))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&update)
                    .expect("serialize deck options update")
                    .expect_json()
                    .build()
                    .then_send(Event::DeckOptionsSaved)
            }

            Event::DeckOptionsSaved(Ok(mut response)) => {
                model.deck_options =
                    Some(response.take_body().expect("deck-options response has a body"));
                model.deck_options_error = None;
                render::render()
            }

            Event::DeckOptionsSaved(Err(e)) => {
                // Keep the currently-loaded preset (so the shell's edited values
                // are not lost) and surface the save error.
                model.deck_options_error = Some(e.to_string());
                render::render()
            }

            Event::DeckOptionsAssign { deck_id, options_id } => {
                // Requires an authenticated token; otherwise a no-op.
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let body = SetDeckOptions { options_id };
                Http::patch(format!("{API_URL}/decks/{deck_id}/options"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize set deck options")
                    .expect_json()
                    .build()
                    .then_send(Event::DeckOptionsAssigned)
            }

            Event::DeckOptionsAssigned(Ok(mut response)) => {
                let deck = response.take_body().expect("set options response has a body");
                // Reflect the new effective `options_id` back into the deck list
                // (so subsequent fetches target the right preset).
                if let Some(existing) = model.decks.iter_mut().find(|d| d.id == deck.id) {
                    existing.options_id = deck.options_id;
                }
                model.deck_options_error = None;
                // Re-fetch the (possibly changed) preset for the updated deck so
                // the shell re-renders the newly-assigned options.
                render::render().and(Command::event(Event::DeckOptionsRequested {
                    deck_id: deck.id,
                }))
            }

            Event::DeckOptionsAssigned(Err(e)) => {
                model.deck_options_error = Some(e.to_string());
                render::render()
            }

            Event::DeckOptionsListRequested => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.presets_error = None;
                Http::get(format!("{API_URL}/deck-options"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::DeckOptionsListResult)
            }

            Event::DeckOptionsListResult(Ok(mut response)) => {
                model.presets =
                    response.take_body().expect("deck-options list response has a body");
                model.presets_error = None;
                render::render()
            }

            Event::DeckOptionsListResult(Err(e)) => {
                model.presets_error = Some(e.to_string());
                render::render()
            }

            Event::TagsRequested => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.tags_error = None;
                Http::get(format!("{API_URL}/tags"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::TagsResult)
            }

            Event::TagsResult(Ok(mut response)) => {
                model.tags = response.take_body().expect("tags response has a body");
                model.tags_error = None;
                render::render()
            }

            Event::TagsResult(Err(e)) => {
                model.tags_error = Some(e.to_string());
                render::render()
            }

            Event::TagTreeRequested => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.tags_error = None;
                Http::get(format!("{API_URL}/tags/tree"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::TagTreeResult)
            }

            Event::TagTreeResult(Ok(mut response)) => {
                model.tag_tree = response.take_body().expect("tag tree response has a body");
                model.tags_error = None;
                render::render()
            }

            Event::TagTreeResult(Err(e)) => {
                model.tags_error = Some(e.to_string());
                render::render()
            }

            Event::NoteTypesRequested => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.note_types_error = None;
                Http::get(format!("{API_URL}/note-types"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::NoteTypesResult)
            }

            Event::NoteTypesResult(Ok(mut response)) => {
                model.note_types = response.take_body().expect("note-types response has a body");
                model.note_types_error = None;
                render::render()
            }

            Event::NoteTypesResult(Err(e)) => {
                model.note_types_error = Some(e.to_string());
                render::render()
            }

            Event::NotesRequested { filter } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.notes_filter = Some(filter.clone());
                model.notes_error = None;
                let query = filter.to_wire_query();
                Http::get(format!("{API_URL}/notes"))
                    .header("authorization", format!("Bearer {token}"))
                    .query(&query)
                    .expect("serialize notes query")
                    .expect_json()
                    .build()
                    .then_send(Event::NotesResult)
            }

            Event::NotesResult(Ok(mut response)) => {
                let page = response.take_body().expect("notes response has a body");
                model.notes = page.notes;
                model.notes_page = page.page;
                model.notes_per_page = page.per_page;
                model.notes_total = page.total;
                model.notes_error = None;
                resync_notes_selected(model);
                render::render()
            }

            Event::NotesResult(Err(e)) => {
                model.notes_error = Some(e.to_string());
                render::render()
            }

            Event::NoteCreate {
                note_type_id,
                deck_id,
                fields,
            } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.notes_error = None;
                let body = CreateNote {
                    note_type_id,
                    deck_id,
                    fields: fields_to_map(fields),
                };
                Http::post(format!("{API_URL}/notes"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize create note")
                    .expect_json()
                    .build()
                    .then_send(Event::NoteCreated)
            }

            Event::NoteCreated(Ok(_)) => {
                model.notes_error = None;
                refresh_notes(model)
            }

            Event::NoteCreated(Err(e)) => {
                model.notes_error = Some(e.to_string());
                render::render()
            }

            Event::NoteSave {
                note_id,
                note_type_id,
                fields,
            } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.notes_error = None;
                let body = UpdateNote {
                    note_type_id,
                    fields: fields.map(fields_to_map),
                };
                Http::patch(format!("{API_URL}/notes/{note_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize update note")
                    .expect_json()
                    .build()
                    .then_send(Event::NoteSaved)
            }

            Event::NoteSaved(Ok(_)) => {
                model.notes_error = None;
                refresh_notes(model)
            }

            Event::NoteSaved(Err(e)) => {
                model.notes_error = Some(e.to_string());
                render::render()
            }

            Event::NoteDelete { note_id } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.notes_error = None;
                Http::delete(format!("{API_URL}/notes/{note_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::NoteDeleted)
            }

            Event::NoteDeleted(Ok(_)) => {
                model.notes_error = None;
                refresh_notes(model)
            }

            Event::NoteDeleted(Err(e)) => {
                model.notes_error = Some(e.to_string());
                render::render()
            }

            Event::StartStudy => {
                // Requires a selected deck (and a token); otherwise a no-op.
                let Some(deck) = &model.selected_deck else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.busy = true;
                model.study_error = None;
                Http::get(format!("{API_URL}/decks/{}/study", deck.id))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::StudyStarted)
            }

            Event::Answer { rating } => {
                // Only answered while a card is shown, and only a valid 1..=4
                // rating does anything; otherwise ignore (no request emitted).
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let Some(deck) = &model.selected_deck else {
                    return render::render();
                };
                if !(1..=4).contains(&rating) {
                    return render::render();
                }
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let body = StudyAdvanceBody {
                    card_id: card.card_id,
                    rating,
                    response_time_ms: None,
                };
                Http::post(format!("{API_URL}/decks/{}/study", deck.id))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize study advance body")
                    .expect_json()
                    .build()
                    .then_send(Event::StudyAdvanced)
            }

            Event::StudyStarted(Ok(mut response)) => {
                let advance = response.take_body().expect("study response has a body");
                model.current_card = advance.next_card;
                model.counts = advance.counts;
                // Fold the fresh per-state due counts back into the selected
                // deck's entry in `model.decks`, so the deck-list badges and the
                // study-screen badges are the *same* number (write-through; no
                // extra `/decks/counts` round-trip).
                fold_study_counts_into_decks(&mut model.decks, model.selected_deck.as_ref(), model.counts);
                model.busy = false;
                model.study_error = None;
                render::render()
            }

            Event::StudyStarted(Err(e)) => {
                model.busy = false;
                model.study_error = Some(e.to_string());
                render::render()
            }

            Event::StudyAdvanced(Ok(mut response)) => {
                let advance = response.take_body().expect("study response has a body");
                model.current_card = advance.next_card;
                model.counts = advance.counts;
                // Same write-through as `StudyStarted`: keep the deck-list
                // badges in sync with the latest study counts.
                fold_study_counts_into_decks(&mut model.decks, model.selected_deck.as_ref(), model.counts);
                model.busy = false;
                model.study_error = None;
                render::render()
            }

            Event::StudyAdvanced(Err(e)) => {
                model.busy = false;
                model.study_error = Some(e.to_string());
                render::render()
            }

            Event::BuryCard => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/bury"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::CardActionDone)
            }

            Event::BuryNote => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let note_id = card.note_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/notes/{note_id}/bury"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::NoteActionDone)
            }

            Event::SuspendCard => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/suspend"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::CardActionDone)
            }

            Event::UnsuspendCard => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/unsuspend"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::CardActionDone)
            }

            Event::UnburyCard => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/unbury"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::CardActionDone)
            }

            Event::SuspendNote => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let note_id = card.note_id;
                model.card_action_error = None;
                Http::post(format!("{API_URL}/notes/{note_id}/suspend"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::NoteActionDone)
            }

            Event::RescheduleCard { days } => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                let body = RescheduleBody {
                    due_at: None,
                    days: Some(days),
                };
                Http::patch(format!("{API_URL}/cards/{card_id}/reschedule"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize reschedule body")
                    .expect_json()
                    .build()
                    .then_send(Event::RescheduleDone)
            }

            Event::FlagCard { flag } => {
                let Some(card) = &model.current_card else {
                    return render::render();
                };
                if !(0..=7).contains(&flag) {
                    return render::render();
                }
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                let card_id = card.card_id;
                model.card_action_error = None;
                let body = SetFlag { flag };
                Http::patch(format!("{API_URL}/cards/{card_id}/flag"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize set flag")
                    .expect_json()
                    .build()
                    .then_send(Event::FlagDone)
            }

            // After a card-level bury/suspend/reschedule, the affected card is no
            // longer due (bury/suspend) or has moved (reschedule) — advance by
            // re-fetching the next due card from the study endpoint.
            Event::CardActionDone(Ok(_)) => {
                model.card_action_error = None;
                render::render().and(Command::event(Event::StartStudy))
            }

            Event::CardActionDone(Err(e)) => {
                model.card_action_error = Some(e.to_string());
                render::render()
            }

            Event::NoteActionDone(Ok(_)) => {
                model.card_action_error = None;
                render::render().and(Command::event(Event::StartStudy))
            }

            Event::NoteActionDone(Err(e)) => {
                model.card_action_error = Some(e.to_string());
                render::render()
            }

            Event::RescheduleDone(Ok(_)) => {
                model.card_action_error = None;
                render::render().and(Command::event(Event::StartStudy))
            }

            Event::RescheduleDone(Err(e)) => {
                model.card_action_error = Some(e.to_string());
                render::render()
            }

            Event::FlagDone(Ok(mut response)) => {
                let flag = response.take_body().expect("flag response has a body");
                // Update the shown card's flag in place (flag does not remove the
                // card from the queue, so no re-fetch).
                if let Some(card) = &mut model.current_card {
                    if card.card_id == flag.card_id {
                        card.flag = flag.flag;
                    }
                }
                model.card_action_error = None;
                render::render()
            }

            Event::FlagDone(Err(e)) => {
                model.card_action_error = Some(e.to_string());
                render::render()
            }

            Event::CardsRequested { filter } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_filter = Some(filter.clone());
                model.browser_error = None;
                let query = filter.to_wire_query();
                Http::get(format!("{API_URL}/cards"))
                    .header("authorization", format!("Bearer {token}"))
                    .query(&query)
                    .expect("serialize card browser query")
                    .expect_json()
                    .build()
                    .then_send(Event::CardsResult)
            }

            Event::CardsResult(Ok(mut response)) => {
                let page = response.take_body().expect("cards response has a body");
                model.browser_cards = page.cards;
                model.browser_page = page.page;
                model.browser_per_page = page.per_page;
                model.browser_total = page.total;
                model.browser_error = None;
                resync_browser_selected(model);
                render::render()
            }

            Event::CardsResult(Err(e)) => {
                model.browser_error = Some(e.to_string());
                render::render()
            }

            Event::BrowserSelectCard { card_id } => {
                model.browser_selected =
                    model.browser_cards.iter().find(|c| c.card_id == card_id).cloned();
                render::render()
            }

            Event::NoteBrowserSelectNote { note_id } => {
                model.notes_selected = model.notes.iter().find(|n| n.id == note_id).cloned();
                render::render()
            }

            Event::BrowserSuspendCard { card_id } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/suspend"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserCardActionDone)
            }

            Event::BrowserBuryCard { card_id } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/bury"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserCardActionDone)
            }

            Event::BrowserUnsuspendCard { card_id } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/unsuspend"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserCardActionDone)
            }

            Event::BrowserUnburyCard { card_id } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                Http::post(format!("{API_URL}/cards/{card_id}/unbury"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserCardActionDone)
            }

            Event::BrowserRescheduleCard { card_id, days } => {
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                let body = RescheduleBody {
                    due_at: None,
                    days: Some(days),
                };
                Http::patch(format!("{API_URL}/cards/{card_id}/reschedule"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize reschedule body")
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserRescheduleDone)
            }

            Event::BrowserFlagCard { card_id, flag } => {
                if !(0..=7).contains(&flag) {
                    return render::render();
                }
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                model.browser_error = None;
                let body = SetFlag { flag };
                Http::patch(format!("{API_URL}/cards/{card_id}/flag"))
                    .header("authorization", format!("Bearer {token}"))
                    .body_json(&body)
                    .expect("serialize set flag")
                    .expect_json()
                    .build()
                    .then_send(Event::BrowserFlagDone)
            }

            // Browser action completions refresh the list using the last filter
            // (unlike the study actions, which advance to the next due card).
            Event::BrowserCardActionDone(Ok(_)) => {
                model.browser_error = None;
                refresh_browser(model)
            }

            Event::BrowserCardActionDone(Err(e)) => {
                model.browser_error = Some(e.to_string());
                render::render()
            }

            Event::BrowserRescheduleDone(Ok(_)) => {
                model.browser_error = None;
                refresh_browser(model)
            }

            Event::BrowserRescheduleDone(Err(e)) => {
                model.browser_error = Some(e.to_string());
                render::render()
            }

            Event::BrowserFlagDone(Ok(mut response)) => {
                let flag = response.take_body().expect("flag response has a body");
                if let Some(card) = model.browser_cards.iter_mut().find(|c| c.card_id == flag.card_id) {
                    card.flag = Some(flag.flag);
                }
                model.browser_error = None;
                resync_browser_selected(model);
                render::render()
            }

            Event::BrowserFlagDone(Err(e)) => {
                model.browser_error = Some(e.to_string());
                render::render()
            }

            Event::DecksRequested => {
                model.decks_error = None;
                // Needs the current token; only reachable when authenticated.
                let token = match &model.auth {
                    Auth::Authenticated { token, .. } => token.clone(),
                    Auth::Unauthenticated => return render::render(),
                };
                Http::get(format!("{API_URL}/decks"))
                    .header("authorization", format!("Bearer {token}"))
                    .expect_json()
                    .build()
                    .then_send(Event::DecksResult)
            }

            Event::DecksResult(Ok(mut response)) => {
                model.decks = response.take_body().expect("decks response has a body");
                model.decks_error = None;
                render::render()
            }

            Event::DecksResult(Err(e)) => {
                model.decks_error = Some(e.to_string());
                render::render()
            }

            Event::DecksCountsResult(Ok(mut response)) => {
                let counts = response.take_body().expect("counts response has a body");
                // Merge per-deck counts into the already-loaded deck list.
                for c in counts.decks {
                    if let Some(deck) = model.decks.iter_mut().find(|d| d.id == c.deck_id) {
                        deck.new_count = Some(c.new_count);
                        deck.learning_count = Some(c.learning_count);
                        deck.review_count = Some(c.review_count);
                        deck.total_count = Some(c.total_count);
                    }
                }
                render::render()
            }

            Event::DecksCountsResult(Err(e)) => {
                model.decks_error = Some(e.to_string());
                render::render()
            }

            // Completion events whose only purpose was to run after the KV write
            // (no further state change needed — the render already happened).
            Event::TokenStored(_) | Event::TokenDeleted(_) => Command::done(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let decks = build_deck_tree(&model.decks);
        let current_card = model.current_card.as_ref().map(StudyCardView::from);
        let counts = StudyCountsView::from(model.counts);

        match &model.auth {
            Auth::Authenticated { user, .. } => ViewModel {
                email: user.email.clone(),
                authenticated: true,
                display_name: format!("{} {}", user.first_name, user.last_name),
                permissions: PermissionsView::from(&user.permissions),
                busy: model.busy,
                error: model.error.clone(),
                decks,
                selected_deck: model.selected_deck.clone(),
                selected_deck_view: model.selected_deck_view,
                route: model.route,
                decks_error: model.decks_error.clone(),
                current_card,
                counts,
                study_error: model.study_error.clone(),
                deck_options: model.deck_options.as_ref().map(DeckOptionsView::from),
                deck_options_error: model.deck_options_error.clone(),
                presets: model.presets.iter().map(PresetSummary::from).collect(),
                presets_error: model.presets_error.clone(),
                tags: model.tags.iter().map(TagView::from).collect(),
                tag_tree: model.tag_tree.iter().map(TagNodeView::from).collect(),
                tags_error: model.tags_error.clone(),
                card_action_error: model.card_action_error.clone(),
                browser_cards: model.browser_cards.iter().map(CardBrowserView::from).collect(),
                browser_page: model.browser_page,
                browser_per_page: model.browser_per_page,
                browser_total: model.browser_total,
                browser_selected: model.browser_selected.as_ref().map(CardBrowserView::from),
                browser_error: model.browser_error.clone(),
                note_types: model.note_types.iter().map(NoteTypeView::from).collect(),
                note_types_error: model.note_types_error.clone(),
                notes: model.notes.iter().map(NoteView::from).collect(),
                notes_page: model.notes_page,
                notes_per_page: model.notes_per_page,
                notes_total: model.notes_total,
                notes_selected: model.notes_selected.as_ref().map(NoteView::from),
                notes_error: model.notes_error.clone(),
            },
            Auth::Unauthenticated => ViewModel {
                email: String::new(),
                authenticated: false,
                display_name: String::new(),
                permissions: PermissionsView::default(),
                busy: model.busy,
                error: model.error.clone(),
                decks,
                selected_deck: None,
                selected_deck_view: DeckView::Study,
                route: model.route,
                decks_error: model.decks_error.clone(),
                current_card: None,
                counts: StudyCountsView::default(),
                study_error: None,
                deck_options: None,
                deck_options_error: None,
                presets: Vec::new(),
                presets_error: None,
                tags: Vec::new(),
                tag_tree: Vec::new(),
                tags_error: None,
                card_action_error: None,
                browser_cards: Vec::new(),
                browser_page: 1,
                browser_per_page: 50,
                browser_total: 0,
                browser_selected: None,
                browser_error: None,
                note_types: Vec::new(),
                note_types_error: None,
                notes: Vec::new(),
                notes_page: 1,
                notes_per_page: 50,
                notes_total: 0,
                notes_selected: None,
                notes_error: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anjuman_contracts::UserRole;
    use crux_core::App as _;
    use crux_http::testing::ResponseBuilder;

    fn update(event: Event, model: &mut Model) -> Vec<Effect> {
        let app = Anjuman::default();
        let mut command = app.update(event, model);
        command.effects().collect()
    }

    // --------------------------------------------------------------------
    // US-4.2 — auth flow
    // --------------------------------------------------------------------

    /// `LoginSubmit` emits a POST to `/auth/login` with a JSON `LoginRequest`
    /// body (plus a render).
    #[test]
    fn login_submit_emits_post_with_json_body() {
        let mut model = Model::default();
        let effects = update(
            Event::LoginSubmit {
                email: "a@b.c".to_string(),
                password: "pw".to_string(),
            },
            &mut model,
        );

        assert!(model.busy);
        let http = effects
            .iter()
            .find_map(|e| match e {
                Effect::Http(req) => Some(&req.operation),
                _ => None,
            })
            .expect("expected an Http effect");

        assert_eq!(http.method.as_str(), "POST");
        assert_eq!(http.url.as_str(), "http://127.0.0.1:3000/auth/login");
        // The JSON body carries the credentials.
        let body: LoginRequest =
            serde_json::from_slice(&http.body).expect("login body is JSON");
        assert_eq!(body.email, "a@b.c");
        assert_eq!(body.password, "pw");
    }

    /// A successful login stores `token` + `user` and emits a KV `set` for the
    /// token.
    #[test]
    fn login_success_stores_token_and_emits_kv_set() {
        let mut model = Model::default();
        let response = ResponseBuilder::ok()
            .body(LoginResponse {
                token: "jwt-token".to_string(),
                user: UserResponse {
                    id: 1,
                    email: "a@b.c".to_string(),
                    first_name: "Ada".to_string(),
                    last_name: "Lovelace".to_string(),
                    role: UserRole::Student,
                    school_id: 1,
                    permissions: test_permissions(UserRole::Student),
                },
            })
            .build();
        let effects = update(Event::LoginResult(Ok(response)), &mut model);

        match &model.auth {
            Auth::Authenticated { token, user } => {
                assert_eq!(token, "jwt-token");
                assert_eq!(user.email, "a@b.c");
            }
            other => panic!("expected authenticated, got {other:?}"),
        }
        // KV set effect for the token (alongside a render).
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Set { ref key, .. } if key == TOKEN_KEY))),
            "expected a KeyValue Set effect for the token"
        );
    }

    /// A login rejection leaves the model unauthenticated and surfaces an error.
    #[test]
    fn login_rejection_leaves_unauthenticated() {
        let mut model = Model::default();
        let err = crux_http::testing::rejection::<LoginResponse>(401, "invalid")
            .unwrap_err();
        let effects = update(Event::LoginResult(Err(err)), &mut model);

        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(!model.busy);
        assert!(model.error.is_some());
        assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
    }

    /// `RestoreSession` reads the token from KV.
    #[test]
    fn restore_session_reads_token_from_kv() {
        let mut model = Model::default();
        let effects = update(Event::RestoreSession, &mut model);

        assert!(matches!(
            effects.as_slice(),
            [Effect::KeyValue(op)]
                if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Get { ref key } if key == TOKEN_KEY)
        ));
    }

    /// A stored token triggers a `GET /me` with the `Authorization: Bearer`
    /// header.
    #[test]
    fn stored_token_triggers_me_with_bearer_header() {
        let mut model = Model::default();
        // KV `get` resolves to `DataResult` = `Result<Option<Vec<u8>>, _>`.
        let effects = update(Event::TokenRead(Ok(Some(b"jwt-token".to_vec()))), &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/me");
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header present");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// The server-resolved `permissions` map into `ViewModel.permissions` (so the
    /// shell can gate management affordances), and a student's differ from a
    /// teacher's.
    #[test]
    fn permissions_map_into_view() {
        // Student (the default `authenticated_model`) has no management caps.
        let student = authenticated_model();
        let svm = Anjuman.view(&student);
        assert!(!svm.permissions.can_manage_decks);
        assert!(!svm.permissions.can_administer_school);

        // A teacher has management caps but not administration.
        let mut teacher = authenticated_model();
        if let Auth::Authenticated { user, .. } = &mut teacher.auth {
            user.role = UserRole::Teacher;
            user.permissions = test_permissions(UserRole::Teacher);
        }
        let tvm = Anjuman.view(&teacher);
        assert!(tvm.permissions.can_manage_decks);
        assert!(tvm.permissions.can_manage_notes);
        assert!(!tvm.permissions.can_administer_school);
    }

    /// `/me` success stores the `UserResponse` (with the pending token); `/me`
    /// 401 clears the token and returns to signed-out.
    #[test]
    fn me_success_and_401() {
        // Success path: restore set `pending_token`; `/me` succeeds and combines
        // it with the returned user.
        let mut model = Model {
            pending_token: Some("jwt-token".to_string()),
            ..Model::default()
        };
        let ok = ResponseBuilder::ok()
            .body(UserResponse {
                id: 7,
                email: "me@b.c".to_string(),
                first_name: "Grace".to_string(),
                last_name: "Hopper".to_string(),
                role: UserRole::Teacher,
                school_id: 1,
                permissions: test_permissions(UserRole::Teacher),
            })
            .build();
        let _ = update(Event::MeResult(Ok(ok)), &mut model);
        match &model.auth {
            Auth::Authenticated { token, user } => {
                assert_eq!(token, "jwt-token");
                assert_eq!(user.email, "me@b.c");
            }
            other => panic!("expected authenticated, got {other:?}"),
        }

        // 401 path: clears token + emits KV delete.
        let err = crux_http::testing::rejection::<UserResponse>(401, "expired").unwrap_err();
        let effects = update(Event::MeResult(Err(err)), &mut model);
        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Delete { ref key } if key == TOKEN_KEY))),
            "expected a KeyValue Delete effect for the token"
        );
    }

    /// `Logout` clears the session and emits a KV delete for the token.
    #[test]
    fn logout_clears_session_and_deletes_token() {
        let mut model = Model {
            auth: Auth::Authenticated {
                token: "jwt-token".to_string(),
                user: UserResponse {
                    id: 1,
                    email: "a@b.c".to_string(),
                    first_name: "Ada".to_string(),
                    last_name: "Lovelace".to_string(),
                    role: UserRole::Student,
                    school_id: 1,
                    permissions: test_permissions(UserRole::Student),
                },
            },
            ..Model::default()
        };
        let effects = update(Event::Logout, &mut model);

        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Delete { ref key } if key == TOKEN_KEY))),
            "expected a KeyValue Delete effect for the token"
        );
    }

    // --------------------------------------------------------------------
    // US-4.3 — decks list
    // --------------------------------------------------------------------

    use anjuman_contracts::decks::DeckCounts;
    use chrono::Utc;

    fn deck(id: i64, title: &str) -> DeckResponse {
        deck_with_parent(id, title, None)
    }

    fn deck_with_parent(id: i64, title: &str, parent_id: Option<i64>) -> DeckResponse {
        DeckResponse {
            id,
            school_id: 1,
            title: title.to_string(),
            description: None,
            created_by: 1,
            owner_email: "o@b.c".to_string(),
            owner_first_name: "O".to_string(),
            owner_last_name: "O".to_string(),
            parent_id,
            created_at: Utc::now(),
            options_id: 0,
            new_per_day_mode: None,
            review_per_day_mode: None,
            new_per_day_override: None,
            review_per_day_override: None,
            new_per_day_today_date: None,
            review_per_day_today_date: None,
            new_count: Some(3),
            learning_count: Some(1),
            review_count: Some(5),
            relearning_count: Some(0),
            total_count: Some(9),
            studyable: true,
        }
    }

    /// A `Permissions` object mirroring the server's `permissions_for` mapping
    /// (used to build test `UserResponse`s).
    fn test_permissions(role: UserRole) -> Permissions {
        let manages = matches!(role, UserRole::Admin | UserRole::Teacher);
        let administers = matches!(role, UserRole::Admin);
        Permissions {
            can_administer_school: administers,
            can_manage_users: administers,
            can_manage_decks: manages,
            can_manage_notes: manages,
            can_manage_presets: manages,
            can_manage_note_types: manages,
            can_manage_tags: manages,
            can_manage_classes: manages,
            can_view_dashboard: manages,
            can_view_analytics: manages,
        }
    }

    fn authenticated_model() -> Model {
        Model {
            auth: Auth::Authenticated {
                token: "jwt-token".to_string(),
                user: UserResponse {
                    id: 1,
                    email: "a@b.c".to_string(),
                    first_name: "Ada".to_string(),
                    last_name: "Lovelace".to_string(),
                    role: UserRole::Student,
                    school_id: 1,
                    permissions: test_permissions(UserRole::Student),
                },
            },
            ..Model::default()
        }
    }

    fn browser_card(card_id: i64) -> CardBrowserResponse {
        CardBrowserResponse {
            card_id,
            note_id: 1,
            deck_id: 1,
            deck_title: "Spanish".to_string(),
            template_id: 1,
            front: "front".to_string(),
            back: "back".to_string(),
            note_type_name: "Basic".to_string(),
            note_type_id: 1,
            template_name: "Card 1".to_string(),
            fields: serde_json::Map::new(),
            state: Some("review".to_string()),
            due_at: None,
            stability: None,
            difficulty: None,
            reps: None,
            lapses: None,
            flag: Some(0),
            suspended: Some(false),
            buried_at: None,
            bury_reason: None,
            created_at: chrono::Utc::now(),
            new_card_position: None,
        }
    }

    fn browser_page(cards: Vec<CardBrowserResponse>) -> CardBrowserPage {
        let total = cards.len() as i64;
        CardBrowserPage {
            cards,
            page: 1,
            per_page: 50,
            total,
        }
    }

    fn note_type(id: i64, name: &str) -> NoteTypeResponse {
        NoteTypeResponse {
            id,
            name: name.to_string(),
            field_names: vec!["Front".to_string(), "Back".to_string()],
            sort_field: String::new(),
            styling: String::new(),
            templates: vec![],
            note_count: 0,
        }
    }

    fn note(id: i64) -> NoteResponse {
        NoteResponse {
            id,
            note_type_id: 1,
            note_type_name: "Basic".to_string(),
            fields: [
                ("Front".to_string(), serde_json::Value::String("hola".to_string())),
                ("Back".to_string(), serde_json::Value::String("hello".to_string())),
            ]
            .into_iter()
            .collect(),
            cards: vec![],
            tags: vec![Tag { id: 1, name: "spanish".to_string() }],
            sort_field: Some("hola".to_string()),
            created_at: chrono::Utc::now(),
        }
    }

    /// `DecksRequested` emits a `GET /decks` with the bearer header.
    #[test]
    fn decks_requested_emits_get_decks_with_bearer() {
        let mut model = authenticated_model();
        let effects = update(Event::DecksRequested, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/decks");
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `DecksRequested` while unauthenticated is a no-op render (no request).
    #[test]
    fn decks_requested_unauthenticated_is_noop() {
        let mut model = Model::default();
        let effects = update(Event::DecksRequested, &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// A canned deck list populates the model and is exposed in the view.
    #[test]
    fn decks_result_populates_model_and_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok().body(vec![deck(1, "Spanish"), deck(2, "Maths")]).build();
        let _ = update(Event::DecksResult(Ok(response)), &mut model);

        assert_eq!(model.decks.len(), 2);
        assert_eq!(model.decks[0].title, "Spanish");

        let vm = Anjuman.view(&model);
        assert_eq!(vm.decks.len(), 2);
        let spanish = &vm.decks[0];
        assert_eq!(spanish.title, "Spanish");
        assert_eq!(spanish.new_count, 3);
        assert_eq!(spanish.review_count, 5);
        assert_eq!(spanish.total_count, 9);
    }

    /// Nested decks are exposed as a tree (children under their parent); an
    /// orphan whose parent is absent surfaces at the root.
    #[test]
    fn decks_result_builds_nested_tree() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(vec![
                deck(1, "Spanish"),
                deck_with_parent(2, "Spanish::Verbs", Some(1)),
                deck(3, "Maths"),
                deck_with_parent(9, "Orphan", Some(99)), // parent 99 not in list
            ])
            .build();
        let _ = update(Event::DecksResult(Ok(response)), &mut model);

        let vm = Anjuman.view(&model);

        // Roots are the two parent decks plus the orphan, in input order.
        assert_eq!(vm.decks.len(), 3);
        assert_eq!(vm.decks[0].title, "Spanish");
        assert_eq!(vm.decks[1].title, "Maths");
        assert_eq!(vm.decks[2].title, "Orphan");

        // Spanish has its subdeck nested.
        assert_eq!(vm.decks[0].children.len(), 1);
        assert_eq!(vm.decks[0].children[0].title, "Spanish::Verbs");
    }

    /// A canned counts response merges per-deck counts into the loaded list.
    #[test]
    fn counts_result_merges_into_loaded_decks() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        let counts = DeckCountsResponse {
            decks: vec![DeckCounts {
                deck_id: 1,
                new_count: 12,
                learning_count: 4,
                review_count: 20,
                relearning_count: 2,
                total_count: 38,
            }],
        };
        let response = ResponseBuilder::ok().body(counts).build();
        let _ = update(Event::DecksCountsResult(Ok(response)), &mut model);

        assert_eq!(model.decks[0].new_count, Some(12));
        assert_eq!(model.decks[0].review_count, Some(20));

        // And the view reflects the merged counts.
        let vm = Anjuman.view(&model);
        assert_eq!(vm.decks[0].new_count, 12);
        assert_eq!(vm.decks[0].total_count, 38);
    }

    /// A decks rejection surfaces an error (on the model and in the view)
    /// without crashing.
    #[test]
    fn decks_rejection_sets_error() {
        let mut model = authenticated_model();
        let err = crux_http::testing::rejection::<Vec<DeckResponse>>(500, "boom").unwrap_err();
        let effects = update(Event::DecksResult(Err(err)), &mut model);

        assert!(model.decks_error.is_some());
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));

        let vm = Anjuman.view(&model);
        assert!(vm.decks_error.is_some());
    }

    /// Logout clears the deck list.
    #[test]
    fn logout_clears_decks() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        let _ = update(Event::Logout, &mut model);
        assert!(model.decks.is_empty());
        assert!(model.selected_deck.is_none(), "logout clears selection");
    }

    // --------------------------------------------------------------------
    // US-9.5 routing — core-owned top-level route
    // --------------------------------------------------------------------

    /// The route defaults to the deck list (fresh model + unauthenticated).
    #[test]
    fn route_defaults_to_deck_list() {
        let model = Model::default();
        assert_eq!(model.route, CurrentRoute::DeckList);
        assert_eq!(Anjuman.view(&model).route, CurrentRoute::DeckList);
    }

    /// `ShowCards` clears an open deck and sets the route to `Cards`.
    #[test]
    fn show_cards_sets_route_and_clears_deck() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        model.selected_deck = Some(find_deck(&model.decks.clone(), 1).unwrap());
        model.selected_deck_view = DeckView::Study;

        let _ = update(Event::ShowCards, &mut model);
        assert_eq!(model.route, CurrentRoute::Cards);
        assert!(model.selected_deck.is_none(), "top-level nav clears the open deck");
    }

    /// `ShowNotes` sets the routes to `Notes` and clears the open deck.
    #[test]
    fn show_notes_sets_route_and_clears_deck() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        model.selected_deck = Some(find_deck(&model.decks.clone(), 1).unwrap());

        let _ = update(Event::ShowNotes, &mut model);
        assert_eq!(model.route, CurrentRoute::Notes);
        assert!(model.selected_deck.is_none());
    }

    /// `OpenDeck` sets `route = Study`; `OpenDeckOptions` sets `DeckOptions`;
    /// `CloseDeck`/`ShowDeckList` set `DeckList`.
    #[test]
    fn deck_open_and_close_set_route() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        let _ = update(Event::OpenDeck { deck_id: 1 }, &mut model);
        assert_eq!(model.route, CurrentRoute::Study);

        let _ = update(Event::OpenDeckOptions { deck_id: 1 }, &mut model);
        assert_eq!(model.route, CurrentRoute::DeckOptions);

        let _ = update(Event::CloseDeck, &mut model);
        assert_eq!(model.route, CurrentRoute::DeckList);
    }


    // --------------------------------------------------------------------
    // US-4.4 — open a deck (study entry)
    // --------------------------------------------------------------------

    /// `OpenDeck` records the selection and exposes it in the view.
    #[test]
    fn open_deck_records_selection() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish"), deck(2, "Maths")];

        let effects = update(Event::OpenDeck { deck_id: 2 }, &mut model);
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));

        let selected = model.selected_deck.clone().expect("selected");
        assert_eq!(selected.id, 2);
        assert_eq!(selected.title, "Maths");

        let vm = Anjuman.view(&model);
        assert_eq!(vm.selected_deck.as_ref().map(|d| d.title.clone()), Some("Maths".to_string()));
    }

    /// Opening an unknown deck is a no-op (no selection, no crash).
    #[test]
    fn open_deck_unknown_id_is_noop() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        let _ = update(Event::OpenDeck { deck_id: 999 }, &mut model);
        assert!(model.selected_deck.is_none());
    }

    /// The selection carries the `studyable` flag from the wire, so the shell
    /// can gate the study affordance on a context-only ancestor (US-2.19).
    #[test]
    fn open_deck_carries_studyable() {
        let mut model = authenticated_model();
        let mut parent = deck(1, "Biology 101");
        parent.studyable = false;
        model.decks = vec![parent];

        let _ = update(Event::OpenDeck { deck_id: 1 }, &mut model);
        let selected = model.selected_deck.clone().expect("selected");
        assert!(!selected.studyable);
    }

    // --------------------------------------------------------------------
    // US-4.11 (sub-route) — core-owned deck view (Study vs Options)
    // --------------------------------------------------------------------

    /// `selected_deck_view` defaults to `Study` (a fresh model, and unauthenticated).
    #[test]
    fn selected_deck_view_defaults_to_study() {
        let model = Model::default();
        assert_eq!(model.selected_deck_view, DeckView::Study);
        assert_eq!(Anjuman.view(&model).selected_deck_view, DeckView::Study);
    }

    /// `OpenDeck` records the selection and lands on the Study sub-view.
    #[test]
    fn open_deck_with_view_records_selected_deck_view() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        // Study (OpenDeck) -> Study view.
        let _ = update(Event::OpenDeck { deck_id: 1 }, &mut model);
        assert_eq!(model.selected_deck_view, DeckView::Study);

        // Options (OpenDeckOptions) -> Options view.
        let _ = update(Event::OpenDeckOptions { deck_id: 1 }, &mut model);
        assert_eq!(model.selected_deck_view, DeckView::Options);
        let vm = Anjuman.view(&model);
        assert_eq!(vm.selected_deck_view, DeckView::Options);
    }

    /// Opening into Options fetches the deck-options preset; opening into Study
    /// chains the study-start (both chain their sub-view's request).
    #[test]
    fn open_deck_chains_the_correct_view_request() {
        // Options -> DeckOptionsRequested (chained; the preset fetch is covered
        // by the deck_options_requested_* tests). We assert the view reflects
        // Options and that OpenDeckOptions selects the deck.
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        let _ = update(Event::OpenDeckOptions { deck_id: 1 }, &mut model);
        assert!(model.selected_deck.is_some());
        assert_eq!(model.selected_deck_view, DeckView::Options);
    }

    /// `CloseDeck` clears the selection and resets `selected_deck_view` to Study,
    /// so re-opening always starts on Study (unless opened into Options).
    #[test]
    fn close_deck_resets_selected_deck_view() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        // Open into Options, then close.
        let _ = update(Event::OpenDeckOptions { deck_id: 1 }, &mut model);
        assert_eq!(model.selected_deck_view, DeckView::Options);

        let _ = update(Event::CloseDeck, &mut model);
        assert!(model.selected_deck.is_none());
        assert_eq!(model.selected_deck_view, DeckView::Study);
    }

    // --------------------------------------------------------------------
    // US-4.5 — study session (single-card loop)
    // --------------------------------------------------------------------

    use anjuman_contracts::study::StudyAdvance;

    fn study_card(card_id: i64) -> StudyCard {
        StudyCard {
            card_id,
            note_id: 1,
            front: "front".to_string(),
            back: "back".to_string(),
            styling: "body { color: black; }".to_string(),
            state: "new".to_string(),
            due_at: None,
            stability: 0.0,
            difficulty: 0.0,
            reps: 0,
            lapses: 0,
            leech: false,
            flag: 0,
            suspended: false,
            buried_at: None,
            bury_reason: None,
            step_index: 0,
            predicted_interval: None,
        }
    }

    fn study_advance(next_card: Option<StudyCard>) -> StudyAdvance {
        StudyAdvance {
            next_card,
            reviewed_card: None,
            counts: StudyCounts {
                new_count: 3,
                learning_count: 1,
                review_count: 5,
                relearning_count: 0,
            },
            deck_id: 1,
            deck_title: "Spanish".to_string(),
        }
    }

    /// A model with a selected deck ready to study (US-4.4 already ran).
    fn study_ready_model() -> Model {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];
        model.selected_deck = Some(find_deck(&model.decks.clone(), 1).expect("deck 1"));
        model
    }

    fn card_mod_response() -> CardModResponse {
        CardModResponse {
            card_id: 42,
            suspended: false,
            buried_at: Some(chrono::Utc::now()),
            bury_reason: Some("user_card".to_string()),
            due_at: None,
            state: "review".to_string(),
        }
    }

    fn note_mod_response() -> NoteModResponse {
        NoteModResponse {
            note_id: 1,
            cards_affected: 1,
            card_ids: vec![42],
            suspended: false,
            buried_at: Some(chrono::Utc::now()),
            bury_reason: Some("user_note".to_string()),
        }
    }

    fn flag_response(flag: i64) -> FlagResponse {
        FlagResponse { card_id: 42, flag }
    }

    /// `StartStudy` emits `GET /decks/{id}/study` with the bearer header.
    #[test]
    fn start_study_emits_get_with_bearer() {
        let mut model = study_ready_model();
        let effects = update(Event::StartStudy, &mut model);

        assert!(model.busy, "study fetch sets busy");
        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/decks/1/study"
                );
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `StartStudy` sets `busy`, and every completion arm clears it — so the
    /// shell can distinguish "fetching" from "nothing due".
    #[test]
    fn study_sets_and_clears_busy() {
        // Start sets busy.
        let mut model = study_ready_model();
        let _ = update(Event::StartStudy, &mut model);
        assert!(model.busy);

        // StudyStarted(Ok) clears it.
        let response = ResponseBuilder::ok().body(study_advance(None)).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);
        assert!(!model.busy);

        // StudyStarted(Err) clears it.
        let mut err_model = study_ready_model();
        let _ = update(Event::StartStudy, &mut err_model);
        assert!(err_model.busy);
        let err = crux_http::testing::rejection::<StudyAdvance>(500, "boom").unwrap_err();
        let _ = update(Event::StudyStarted(Err(err)), &mut err_model);
        assert!(!err_model.busy);

        // StudyAdvanced(Ok/Err) clear it too (they run after an answer, where
        // busy may already be false — the clear is idempotent).
        let mut ans = study_ready_model();
        ans.current_card = Some(study_card(42));
        let adv_ok = ResponseBuilder::ok().body(study_advance(Some(study_card(43)))).build();
        let _ = update(Event::StudyAdvanced(Ok(adv_ok)), &mut ans);
        assert!(!ans.busy);

        let mut ans_err = study_ready_model();
        ans_err.current_card = Some(study_card(42));
        let e = crux_http::testing::rejection::<StudyAdvance>(500, "boom").unwrap_err();
        let _ = update(Event::StudyAdvanced(Err(e)), &mut ans_err);
        assert!(!ans_err.busy);
    }

    /// `Answer` emits `POST /decks/{id}/study` with a `StudyAdvanceBody` carrying
    /// the current card id + rating.
    #[test]
    fn answer_emits_post_with_body() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));

        let effects = update(Event::Answer { rating: 3 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/decks/1/study"
                );
                let body: StudyAdvanceBody =
                    serde_json::from_slice(&req.operation.body).expect("body is JSON");
                assert_eq!(body.card_id, 42);
                assert_eq!(body.rating, 3);
                assert_eq!(body.response_time_ms, None);
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A study response populates `current_card` + `counts` (and the view).
    #[test]
    fn study_populates_current_card() {
        let mut model = study_ready_model();
        let response = ResponseBuilder::ok()
            .body(study_advance(Some(study_card(7))))
            .build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        let card = model.current_card.as_ref().expect("current card");
        assert_eq!(card.card_id, 7);
        assert_eq!(model.counts.new_count, 3);
        assert!(model.study_error.is_none());

        let vm = Anjuman.view(&model);
        assert_eq!(vm.current_card.as_ref().map(|c| c.card_id), Some(7));
        assert_eq!(vm.counts.review_count, 5);
    }

    /// `StudyStarted(Ok)` folds the fresh counts back into the selected deck's
    /// entry in `model.decks` (write-through), so deck-list badges match the
    /// study-screen badges.
    #[test]
    fn study_started_updates_current_deck_counts() {
        let mut model = study_ready_model();
        // The deck list starts with new=3/learning=1/review=5/relearning=0.
        assert_eq!(model.decks[0].review_count, Some(5));

        // A study response carrying *different* counts (e.g. one card answered).
        let mut advance = study_advance(Some(study_card(7)));
        advance.counts = StudyCounts {
            new_count: 2,
            learning_count: 3,
            review_count: 4,
            relearning_count: 1,
        };
        let response = ResponseBuilder::ok().body(advance).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        // The study counts are the new authoritative source.
        assert_eq!(model.counts.review_count, 4);
        // ...and the deck-list entry now reflects the same numbers.
        assert_eq!(model.decks[0].new_count, Some(2));
        assert_eq!(model.decks[0].learning_count, Some(3));
        assert_eq!(model.decks[0].review_count, Some(4));
        assert_eq!(model.decks[0].relearning_count, Some(1));
        // total_count is a card count, not a due count — untouched.
        assert_eq!(model.decks[0].total_count, Some(9));
    }

    /// `StudyAdvanced(Ok)` (after answering) also folds the new counts into the
    /// selected deck, keeping deck-list badges fresh without a refetch.
    #[test]
    fn study_advanced_updates_current_deck_counts() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));

        let mut advance = study_advance(Some(study_card(43)));
        advance.counts = StudyCounts {
            new_count: 0,
            learning_count: 0,
            review_count: 3,
            relearning_count: 0,
        };
        let response = ResponseBuilder::ok().body(advance).build();
        let _ = update(Event::StudyAdvanced(Ok(response)), &mut model);

        assert_eq!(model.decks[0].new_count, Some(0));
        assert_eq!(model.decks[0].learning_count, Some(0));
        assert_eq!(model.decks[0].review_count, Some(3));
        assert_eq!(model.decks[0].relearning_count, Some(0));
        // total_count unchanged.
        assert_eq!(model.decks[0].total_count, Some(9));
    }

    /// When no deck is selected (or the selected id is absent from the list),
    /// folding is a no-op — the study counts update but nothing panics.
    #[test]
    fn fold_study_counts_noop_without_matching_deck() {
        let mut model = authenticated_model(); // no selected_deck
        let advance = study_advance(Some(study_card(7)));
        let response = ResponseBuilder::ok().body(advance).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        assert_eq!(model.counts.new_count, 3);
        assert!(model.decks.is_empty());
    }

    /// `StudyCard.predicted_interval` (rating→seconds, string-keyed on the wire)
    /// is mapped into `StudyCardView` keyed by `i32` rating.
    #[test]
    fn study_maps_predicted_interval() {
        let mut model = study_ready_model();
        let mut card = study_card(7);
        card.predicted_interval = Some(
            [("1".to_string(), 60), ("2".to_string(), 600), ("3".to_string(), 1209600), ("4".to_string(), 2073600)]
                .into_iter()
                .collect(),
        );
        let response = ResponseBuilder::ok().body(study_advance(Some(card))).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        let vm = Anjuman.view(&model);
        let intervals = vm
            .current_card
            .as_ref()
            .and_then(|c| c.predicted_interval.as_ref())
            .expect("predicted_interval mapped into view");
        assert_eq!(intervals.get(&1), Some(&60));
        assert_eq!(intervals.get(&2), Some(&600));
        assert_eq!(intervals.get(&3), Some(&1209600));
        assert_eq!(intervals.get(&4), Some(&2073600));
    }

    /// `StudyCard.styling` is mapped into `StudyCardView.styling` so the shell
    /// can inject it into the card.
    #[test]
    fn study_maps_styling() {
        let mut model = study_ready_model();
        let mut card = study_card(7);
        card.styling = "body { color: red; }".to_string();
        let response = ResponseBuilder::ok().body(study_advance(Some(card))).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        let vm = Anjuman.view(&model);
        assert_eq!(
            vm.current_card.as_ref().map(|c| c.styling.as_str()),
            Some("body { color: red; }")
        );
    }

    /// A finished study session (`next_card: None`) exposes a "nothing due" state
    /// (empty `current_card` in the view), not a panic.
    #[test]
    fn study_finished_when_next_card_none() {
        let mut model = study_ready_model();
        let response = ResponseBuilder::ok().body(study_advance(None)).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        assert!(model.current_card.is_none());

        let vm = Anjuman.view(&model);
        assert!(vm.current_card.is_none(), "finished state has no current card");
    }

    /// Answering with an invalid rating (or with no current card) is ignored —
    /// no request is emitted.
    #[test]
    fn answer_invalid_rating_is_ignored() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));

        // Out-of-range rating.
        let effects = update(Event::Answer { rating: 5 }, &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));

        // No current card to answer.
        let mut empty = study_ready_model();
        let effects2 = update(Event::Answer { rating: 3 }, &mut empty);
        assert!(!effects2.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// A study error (start or advance) surfaces in `study_error` and the view.
    #[test]
    fn study_error_surfaces() {
        let mut model = study_ready_model();
        let err = crux_http::testing::rejection::<StudyAdvance>(500, "boom").unwrap_err();
        let _ = update(Event::StudyStarted(Err(err)), &mut model);

        assert!(model.study_error.is_some());
        let vm = Anjuman.view(&model);
        assert!(vm.study_error.is_some());
    }

    /// Logout clears the in-flight study state.
    #[test]
    fn logout_clears_study_state() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        model.counts = StudyCounts {
            new_count: 9,
            ..Default::default()
        };
        let _ = update(Event::Logout, &mut model);
        assert!(model.current_card.is_none());
        assert_eq!(model.counts.new_count, 0);
    }

    /// `CloseDeck` clears the selection + study state but keeps auth + decks
    /// (back-navigation, distinct from `Logout`).
    #[test]
    fn close_deck_clears_selection_but_keeps_session() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        model.counts = StudyCounts {
            new_count: 9,
            ..Default::default()
        };
        model.study_error = Some("boom".to_string());

        let effects = update(Event::CloseDeck, &mut model);

        assert!(model.selected_deck.is_none(), "selection cleared");
        assert!(model.current_card.is_none(), "card cleared");
        assert_eq!(model.counts.new_count, 0, "counts reset");
        assert!(model.study_error.is_none(), "study error reset");
        // Auth + deck data are preserved (unlike Logout).
        assert!(matches!(model.auth, Auth::Authenticated { .. }));
        assert_eq!(model.decks.len(), 1);
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));
    }

    // --------------------------------------------------------------------
    // US-4.9 — deck-options settings screen (view a preset)
    // --------------------------------------------------------------------

    use anjuman_contracts::deck_options::DeckOptions;

    fn deck_options(id: i64, name: &str) -> DeckOptions {
        DeckOptions {
            id,
            school_id: 1,
            name: name.to_string(),
            learning_steps: vec![60, 600],
            relearning_steps: vec![600],
            desired_retention: 0.9,
            bury_new: false,
            bury_review: false,
            bury_interday: false,
            new_per_day: 20,
            review_per_day: 200,
            leech_threshold: 8,
            leech_action: anjuman_contracts::deck_options::LeechAction::SuspendCard,
            new_gather_order: anjuman_contracts::deck_options::NewGatherOrder::Deck,
            new_sort_order: anjuman_contracts::deck_options::NewSortOrder::CardTypeThenGathered,
            new_review_order: anjuman_contracts::deck_options::NewReviewOrder::Mix,
            interday_order: anjuman_contracts::deck_options::InterdayOrder::Mix,
            review_sort_order: anjuman_contracts::deck_options::ReviewSortOrder::DueThenRandom,
            show_on_screen_timer: false,
            stop_timer_on_answer: false,
            dont_play_audio_automatically: false,
            skip_question_when_replaying_answer: false,
            auto_advance_seconds_show_question: 0.0,
            auto_advance_seconds_show_answer: 0.0,
            auto_advance_wait_for_audio: true,
            auto_advance_question_action:
                anjuman_contracts::deck_options::AutoAdvanceQuestionAction::ShowAnswer,
            auto_advance_answer_action:
                anjuman_contracts::deck_options::AutoAdvanceAnswerAction::BuryCard,
            maximum_answer_seconds: 60,
            maximum_interval: 36500,
            easy_days: vec![anjuman_contracts::deck_options::EasyDayStrength::Normal; 7],
            insertion_order: anjuman_contracts::deck_options::InsertionOrder::Sequential,
            fsrs_parameters: vec![],
        }
    }

    /// `DeckOptionsRequested` resolves the deck's `options_id` and emits
    /// `GET /deck-options/{id}` with the bearer header.
    #[test]
    fn deck_options_requested_emits_get_with_bearer() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")]; // fixture has options_id: 0

        let effects = update(Event::DeckOptionsRequested { deck_id: 1 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/deck-options/0"
                );
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A deck with a concrete `options_id` (not the 0 default) targets that id.
    #[test]
    fn deck_options_requested_uses_effective_id() {
        let mut model = authenticated_model();
        let mut d = deck(1, "Spanish");
        d.options_id = 42;
        model.decks = vec![d];

        let effects = update(Event::DeckOptionsRequested { deck_id: 1 }, &mut model);
        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/deck-options/42"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// Requesting options for an unknown deck is a no-op (no request emitted).
    #[test]
    fn deck_options_requested_unknown_deck_is_noop() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        let effects = update(Event::DeckOptionsRequested { deck_id: 999 }, &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// A canned `DeckOptions` response populates the model and exposes its
    /// values in the view (enums mapped to their snake_case wire names).
    #[test]
    fn deck_options_result_populates_model_and_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(deck_options(0, "Default"))
            .build();
        let _ = update(Event::DeckOptionsResult(Ok(response)), &mut model);

        assert!(model.deck_options_error.is_none());
        let vm = Anjuman.view(&model);
        let opts = vm.deck_options.as_ref().expect("deck options in view");
        assert_eq!(opts.name, "Default");
        assert_eq!(opts.new_per_day, 20);
        assert_eq!(opts.review_per_day, 200);
        assert_eq!(opts.learning_steps, vec![60, 600]);
        // Enums map to their serde snake_case names.
        assert_eq!(opts.new_gather_order, "deck");
        assert_eq!(opts.review_sort_order, "due_then_random");
        assert_eq!(opts.leech_action, "suspend_card");
        assert_eq!(
            opts.auto_advance_question_action,
            "show_answer"
        );
    }

    /// A deck-options rejection surfaces an error (model + view) without
    /// crashing.
    #[test]
    fn deck_options_rejection_sets_error() {
        let mut model = authenticated_model();
        let err = crux_http::testing::rejection::<DeckOptions>(500, "boom").unwrap_err();
        let _ = update(Event::DeckOptionsResult(Err(err)), &mut model);

        assert!(model.deck_options.is_none());
        assert!(model.deck_options_error.is_some());
        let vm = Anjuman.view(&model);
        assert!(vm.deck_options_error.is_some());
    }

    /// `CloseDeck` clears the fetched preset (back-navigation resets state).
    #[test]
    fn close_deck_clears_deck_options() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok().body(deck_options(0, "Default")).build();
        let _ = update(Event::DeckOptionsResult(Ok(response)), &mut model);
        assert!(model.deck_options.is_some());

        let _ = update(Event::CloseDeck, &mut model);
        assert!(model.deck_options.is_none());
    }

    // --------------------------------------------------------------------
    // US-4.10 — deck-options settings screen (edit + save a preset)
    // --------------------------------------------------------------------

    /// A model with a preset already loaded (US-4.9 ran), ready to save edits.
    fn loaded_preset_model(id: i64) -> Model {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok().body(deck_options(id, "Default")).build();
        let _ = update(Event::DeckOptionsResult(Ok(response)), &mut model);
        model
    }

    /// `DeckOptionsSave` emits `PATCH /deck-options/{id}` with an
    /// `UpdateDeckOptions` body carrying only the changed (`Some`) fields.
    #[test]
    fn deck_options_save_emits_patch_with_body() {
        let mut model = loaded_preset_model(7);
        let edit = DeckOptionsEdit {
            new_per_day: Some(30),
            desired_retention: Some(0.85),
            ..DeckOptionsEdit::default()
        };

        let effects = update(Event::DeckOptionsSave(edit), &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/deck-options/7"
                );
                let body: UpdateDeckOptions =
                    serde_json::from_slice(&req.operation.body).expect("body is JSON");
                // Only the changed fields are present; unchanged are None.
                assert_eq!(body.new_per_day, Some(30));
                assert_eq!(body.desired_retention, Some(0.85));
                assert_eq!(body.review_per_day, None);
                assert_eq!(body.name, None);
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `DeckOptionsSave` with no loaded preset (or unauthenticated) is a no-op.
    #[test]
    fn deck_options_save_without_loaded_preset_is_noop() {
        let mut model = authenticated_model(); // no preset loaded
        let edit = DeckOptionsEdit {
            new_per_day: Some(30),
            ..DeckOptionsEdit::default()
        };
        let effects = update(Event::DeckOptionsSave(edit), &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// A successful save updates the loaded preset; a rejection surfaces an
    /// error and keeps the previously-loaded preset (edited values not lost).
    #[test]
    fn deck_options_save_success_and_rejection() {
        // Success: the PATCH response replaces the loaded preset.
        let mut model = loaded_preset_model(7);
        let updated = ResponseBuilder::ok().body(deck_options(7, "Renamed")).build();
        let _ = update(Event::DeckOptionsSaved(Ok(updated)), &mut model);
        assert_eq!(model.deck_options.as_ref().map(|o| o.name.as_str()), Some("Renamed"));
        assert!(model.deck_options_error.is_none());

        // Rejection: error set, but the loaded preset is kept.
        let mut model2 = loaded_preset_model(7);
        let err = crux_http::testing::rejection::<DeckOptions>(500, "boom").unwrap_err();
        let _ = update(Event::DeckOptionsSaved(Err(err)), &mut model2);
        assert!(model2.deck_options_error.is_some());
        assert_eq!(model2.deck_options.as_ref().map(|o| o.id), Some(7), "preset kept on error");
    }

    /// Local validation rejects out-of-range values before any request is
    /// emitted (the error is surfaced on the model, not a failed save).
    #[test]
    fn deck_options_save_validates_ranges_before_emitting() {
        // new_per_day out of range (must be 0..=9999).
        let mut model = loaded_preset_model(7);
        let bad = DeckOptionsEdit {
            new_per_day: Some(10_000),
            ..DeckOptionsEdit::default()
        };
        let effects = update(Event::DeckOptionsSave(bad), &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
        assert!(model.deck_options_error.is_some());

        // leech_threshold below min (must be 1..=9999).
        let mut model2 = loaded_preset_model(7);
        let bad2 = DeckOptionsEdit {
            leech_threshold: Some(0),
            ..DeckOptionsEdit::default()
        };
        let effects2 = update(Event::DeckOptionsSave(bad2), &mut model2);
        assert!(!effects2.iter().any(|e| matches!(e, Effect::Http(_))));
        assert!(model2.deck_options_error.is_some());

        // desired_retention out of range (must be 0.70..=0.99).
        let mut model3 = loaded_preset_model(7);
        let bad3 = DeckOptionsEdit {
            desired_retention: Some(0.5),
            ..DeckOptionsEdit::default()
        };
        let effects3 = update(Event::DeckOptionsSave(bad3), &mut model3);
        assert!(!effects3.iter().any(|e| matches!(e, Effect::Http(_))));
        assert!(model3.deck_options_error.is_some());

        // A valid value still emits the PATCH.
        let mut model4 = loaded_preset_model(7);
        let good = DeckOptionsEdit {
            new_per_day: Some(9999),
            ..DeckOptionsEdit::default()
        };
        let ok_effects = update(Event::DeckOptionsSave(good), &mut model4);
        assert!(ok_effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    // --------------------------------------------------------------------
    // US-4.11 — assign a deck's preset
    // --------------------------------------------------------------------

    /// `DeckOptionsAssign` emits a `PATCH /decks/{id}/options` with a JSON
    /// `SetDeckOptions { options_id }` body + bearer header.
    #[test]
    fn deck_options_assign_emits_patch_with_body() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")];

        let effects = update(
            Event::DeckOptionsAssign {
                deck_id: 1,
                options_id: 42,
            },
            &mut model,
        );

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/decks/1/options"
                );
                let body: SetDeckOptions =
                    serde_json::from_slice(&req.operation.body).expect("body is JSON");
                assert_eq!(body.options_id, 42);
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `DeckOptionsListRequested` emits `GET /deck-options` with the bearer
    /// header.
    #[test]
    fn deck_options_list_requested_emits_get() {
        let mut model = authenticated_model();
        let effects = update(Event::DeckOptionsListRequested, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/deck-options"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A canned preset list populates the model and is exposed in the view.
    #[test]
    fn deck_options_list_result_populates_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(vec![deck_options(5, "Medical"), deck_options(6, "Language")])
            .build();
        let _ = update(Event::DeckOptionsListResult(Ok(response)), &mut model);

        assert!(model.presets_error.is_none());
        let vm = Anjuman.view(&model);
        assert_eq!(vm.presets.len(), 2);
        assert_eq!(vm.presets[0].id, 5);
        assert_eq!(vm.presets[0].name, "Medical");
        assert_eq!(vm.presets[1].name, "Language");
    }

    /// Assignment success updates the deck's effective `options_id` in the model
    /// and re-renders (the preset re-fetch is chained as a follow-up event).
    #[test]
    fn deck_options_assign_success_updates_deck_and_refetches() {
        let mut model = authenticated_model();
        model.decks = vec![deck(1, "Spanish")]; // options_id: 0

        // The server responds with the updated deck (now preset 42).
        let mut updated_deck = deck(1, "Spanish");
        updated_deck.options_id = 42;
        let response = ResponseBuilder::ok().body(updated_deck).build();
        let effects = update(Event::DeckOptionsAssigned(Ok(response)), &mut model);

        // The deck's effective options_id is now 42.
        assert_eq!(model.decks[0].options_id, 42);
        assert!(model.deck_options_error.is_none());
        // A render is emitted; the preset re-fetch is chained as a follow-up
        // event (`DeckOptionsRequested`), not an effect in this call.
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));
    }

    // --------------------------------------------------------------------
    // US-6.5a — tag list + tree in the client core
    // --------------------------------------------------------------------

    /// `TagsRequested` emits `GET /tags` with the bearer header.
    #[test]
    fn tags_requested_emits_get_with_bearer() {
        let mut model = authenticated_model();
        let effects = update(Event::TagsRequested, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/tags");
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `TagsRequested` while unauthenticated is a no-op render (no request).
    #[test]
    fn tags_requested_unauthenticated_is_noop() {
        let mut model = Model::default();
        let effects = update(Event::TagsRequested, &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// `TagTreeRequested` emits `GET /tags/tree` with the bearer header.
    #[test]
    fn tag_tree_requested_emits_get() {
        let mut model = authenticated_model();
        let effects = update(Event::TagTreeRequested, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/tags/tree"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A canned tag list populates the model and is exposed in the view.
    #[test]
    fn tags_result_populates_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(vec![
                Tag { id: 1, name: "biology".to_string() },
                Tag { id: 2, name: "biology::cells".to_string() },
            ])
            .build();
        let _ = update(Event::TagsResult(Ok(response)), &mut model);

        assert!(model.tags_error.is_none());
        let vm = Anjuman.view(&model);
        assert_eq!(vm.tags.len(), 2);
        assert_eq!(vm.tags[0].id, 1);
        assert_eq!(vm.tags[0].name, "biology");
        assert_eq!(vm.tags[1].name, "biology::cells");
    }

    /// A canned tag tree populates the model (nested) and is exposed in the view.
    #[test]
    fn tag_tree_result_populates_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(vec![TagNode {
                name: "biology".to_string(),
                children: vec![TagNode { name: "cells".to_string(), children: vec![] }],
            }])
            .build();
        let _ = update(Event::TagTreeResult(Ok(response)), &mut model);

        assert!(model.tags_error.is_none());
        let vm = Anjuman.view(&model);
        assert_eq!(vm.tag_tree.len(), 1);
        assert_eq!(vm.tag_tree[0].name, "biology");
        assert_eq!(vm.tag_tree[0].children.len(), 1);
        assert_eq!(vm.tag_tree[0].children[0].name, "cells");
    }

    /// A tag-list error surfaces in `tags_error` and leaves tags empty.
    #[test]
    fn tags_result_error_sets_tags_error() {
        let mut model = authenticated_model();
        let err = crux_http::testing::rejection::<Vec<Tag>>(500, "boom").unwrap_err();
        let _ = update(Event::TagsResult(Err(err)), &mut model);

        assert!(model.tags_error.is_some());
        let vm = Anjuman.view(&model);
        assert!(vm.tags_error.is_some());
        assert!(vm.tags.is_empty());
    }

    /// `Logout` clears the tag list, tree and error state.
    #[test]
    fn logout_clears_tags_state() {
        let mut model = authenticated_model();
        model.tags = vec![Tag { id: 1, name: "biology".to_string() }];
        model.tag_tree = vec![TagNode { name: "biology".to_string(), children: vec![] }];
        model.tags_error = Some("boom".to_string());

        let _ = update(Event::Logout, &mut model);

        assert!(model.tags.is_empty());
        assert!(model.tag_tree.is_empty());
        assert!(model.tags_error.is_none());
    }

    // --------------------------------------------------------------------
    // US-4.5b — study actions (bury / suspend / reschedule / flag)
    // --------------------------------------------------------------------

    /// `BuryCard` emits `POST /cards/{id}/bury` with the bearer header.
    #[test]
    fn bury_card_emits_post() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::BuryCard, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/bury"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BuryNote` emits `POST /notes/{note_id}/bury` (using the current card's
    /// note id).
    #[test]
    fn bury_note_emits_post() {
        let mut model = study_ready_model();
        let mut card = study_card(42);
        card.note_id = 7;
        model.current_card = Some(card);
        let effects = update(Event::BuryNote, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/notes/7/bury"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `SuspendCard` emits `POST /cards/{id}/suspend`.
    #[test]
    fn suspend_card_emits_post() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::SuspendCard, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/suspend"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `UnsuspendCard` emits `POST /cards/{id}/unsuspend`.
    #[test]
    fn unsuspend_card_emits_post() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::UnsuspendCard, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/unsuspend"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `UnburyCard` emits `POST /cards/{id}/unbury`.
    #[test]
    fn unbury_card_emits_post() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::UnburyCard, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/unbury"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `SuspendNote` emits `POST /notes/{note_id}/suspend`.
    #[test]
    fn suspend_note_emits_post() {
        let mut model = study_ready_model();
        let mut card = study_card(42);
        card.note_id = 7;
        model.current_card = Some(card);
        let effects = update(Event::SuspendNote, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/notes/7/suspend"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `RescheduleCard { days }` emits `PATCH /cards/{id}/reschedule` with a
    /// `RescheduleBody { days }` body.
    #[test]
    fn reschedule_card_emits_patch_with_body() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::RescheduleCard { days: 3 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/reschedule"
                );
                let body: RescheduleBody =
                    serde_json::from_slice(&req.operation.body).expect("body is JSON");
                assert_eq!(body.days, Some(3));
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `FlagCard { flag }` emits `PATCH /cards/{id}/flag` with a `SetFlag` body.
    #[test]
    fn flag_card_emits_patch_with_body() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::FlagCard { flag: 3 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/flag"
                );
                let body: SetFlag =
                    serde_json::from_slice(&req.operation.body).expect("body is JSON");
                assert_eq!(body.flag, 3);
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A card-level action has no-op (no request) when no card is shown, or the
    /// rating/flag is out of range.
    #[test]
    fn card_action_without_current_card_is_noop() {
        let mut model = study_ready_model(); // current_card: None
        for ev in [
            Event::BuryCard,
            Event::BuryNote,
            Event::SuspendCard,
            Event::SuspendNote,
            Event::RescheduleCard { days: 1 },
        ] {
            let effects = update(ev, &mut model);
            assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
        }

        // Flag out of range is ignored even with a current card.
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let effects = update(Event::FlagCard { flag: 8 }, &mut model);
        assert!(!effects.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// `FlagDone(Ok)` updates the current card's flag in place and re-renders.
    #[test]
    fn flag_done_updates_current_card_flag() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42)); // flag 0
        let response = ResponseBuilder::ok().body(flag_response(3)).build();
        let effects = update(Event::FlagDone(Ok(response)), &mut model);

        assert_eq!(model.current_card.as_ref().map(|c| c.flag), Some(3));
        assert!(model.card_action_error.is_none());
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));
    }

    /// A card action error surfaces in `card_action_error` (and the view).
    #[test]
    fn card_action_error_surfaces() {
        let mut model = study_ready_model();
        model.current_card = Some(study_card(42));
        let err = crux_http::testing::rejection::<CardModResponse>(500, "boom").unwrap_err();
        let _ = update(Event::CardActionDone(Err(err)), &mut model);

        assert!(model.card_action_error.is_some());
        let vm = Anjuman.view(&model);
        assert!(vm.card_action_error.is_some());
    }

    /// `note_id` and `flag` are mapped into `StudyCardView` for the shell.
    #[test]
    fn study_maps_note_id_and_flag() {
        let mut model = study_ready_model();
        let mut card = study_card(7);
        card.note_id = 99;
        card.flag = 4;
        let response = ResponseBuilder::ok().body(study_advance(Some(card))).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        let vm = Anjuman.view(&model);
        let card_view = vm.current_card.as_ref().expect("current card");
        assert_eq!(card_view.note_id, 99);
        assert_eq!(card_view.flag, 4);
    }

    /// `StudyCard.suspended`/`buried_at` are mapped into `StudyCardView.suspended`/
    /// `buried` so the study screen can gate `UnsuspendCard`/`UnburyCard`.
    #[test]
    fn study_maps_suspended_and_buried() {
        let mut model = study_ready_model();
        let mut card = study_card(7);
        card.suspended = true;
        card.buried_at = Some(chrono::Utc::now());
        let response = ResponseBuilder::ok().body(study_advance(Some(card))).build();
        let _ = update(Event::StudyStarted(Ok(response)), &mut model);

        let vm = Anjuman.view(&model);
        let card_view = vm.current_card.as_ref().expect("current card");
        assert!(card_view.suspended);
        assert!(card_view.buried);

        // An unsuspended/unburied card maps both to false.
        let mut card2 = study_card(8);
        card2.suspended = false;
        card2.buried_at = None;
        let response2 = ResponseBuilder::ok().body(study_advance(Some(card2))).build();
        let _ = update(Event::StudyStarted(Ok(response2)), &mut model);
        let card_view2 = Anjuman.view(&model).current_card.expect("current card");
        assert!(!card_view2.suspended);
        assert!(!card_view2.buried);
    }

    // --------------------------------------------------------------------
    // US-4.12 — card browser
    // --------------------------------------------------------------------

    /// `CardsRequested` emits `GET /cards` with the folded query + bearer header.
    #[test]
    fn cards_requested_emits_get_with_query() {
        let mut model = authenticated_model();
        let filter = CardBrowserFilter {
            deck_ids: vec![1, 2],
            note_type_ids: vec![7],
            q: Some("DNA".to_string()),
            states: vec!["review".to_string()],
            flags: vec![1, 3],
            sort: "due_at".to_string(),
            page: 2,
            per_page: 25,
        };
        let effects = update(Event::CardsRequested { filter }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                let url = req.operation.url.to_string();
                assert!(url.starts_with("http://127.0.0.1:3000/cards?"), "url = {url}");
                assert!(url.contains("deck_id=1,2"), "url = {url}");
                assert!(url.contains("note_type_id=7"), "url = {url}");
                assert!(url.contains("q=DNA"), "url = {url}");
                assert!(url.contains("state=review"), "url = {url}");
                assert!(url.contains("flag=1,3"), "url = {url}");
                assert!(url.contains("sort=due_at"), "url = {url}");
                assert!(url.contains("page=2"), "url = {url}");
                assert!(url.contains("per_page=25"), "url = {url}");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `CardsResult(Ok)` populates the browser list + page info and the view.
    #[test]
    fn cards_result_populates_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(browser_page(vec![browser_card(11), browser_card(12)]))
            .build();
        let _ = update(Event::CardsResult(Ok(response)), &mut model);

        assert_eq!(model.browser_cards.len(), 2);
        assert_eq!(model.browser_total, 2);
        assert!(model.browser_error.is_none());

        let vm = Anjuman.view(&model);
        assert_eq!(vm.browser_cards.len(), 2);
        assert_eq!(vm.browser_cards[0].card_id, 11);
        assert_eq!(vm.browser_cards[0].deck_title, "Spanish");
    }

    /// `BrowserSelectCard` records the selected card; an unknown id clears it.
    #[test]
    fn browser_select_card_records_selection() {
        let mut model = authenticated_model();
        model.browser_cards = vec![browser_card(11), browser_card(12)];

        let _ = update(Event::BrowserSelectCard { card_id: 12 }, &mut model);
        assert_eq!(model.browser_selected.as_ref().map(|c| c.card_id), Some(12));

        let _ = update(Event::BrowserSelectCard { card_id: 999 }, &mut model);
        assert!(model.browser_selected.is_none());
    }

    /// `BrowserSuspendCard` emits `POST /cards/{id}/suspend`.
    #[test]
    fn browser_suspend_emits_post() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserSuspendCard { card_id: 42 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/suspend"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BrowserBuryCard` emits `POST /cards/{id}/bury`.
    #[test]
    fn browser_bury_emits_post() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserBuryCard { card_id: 42 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/bury"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BrowserUnsuspendCard` emits `POST /cards/{id}/unsuspend`.
    #[test]
    fn browser_unsuspend_emits_post() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserUnsuspendCard { card_id: 42 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/unsuspend"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BrowserUnburyCard` emits `POST /cards/{id}/unbury`.
    #[test]
    fn browser_unbury_emits_post() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserUnburyCard { card_id: 42 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/unbury"
                );
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BrowserRescheduleCard` emits `PATCH /cards/{id}/reschedule` with a body.
    #[test]
    fn browser_reschedule_emits_patch_with_body() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserRescheduleCard { card_id: 42, days: 3 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/reschedule"
                );
                let body: RescheduleBody =
                    serde_json::from_slice(&req.operation.body).expect("body JSON");
                assert_eq!(body.days, Some(3));
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `BrowserFlagCard` emits `PATCH /cards/{id}/flag`; out-of-range is a no-op.
    #[test]
    fn browser_flag_emits_patch_and_rejects_out_of_range() {
        let mut model = authenticated_model();
        let effects = update(Event::BrowserFlagCard { card_id: 42, flag: 3 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/cards/42/flag"
                );
                let body: SetFlag =
                    serde_json::from_slice(&req.operation.body).expect("body JSON");
                assert_eq!(body.flag, 3);
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }

        let noop = update(Event::BrowserFlagCard { card_id: 42, flag: 8 }, &mut model);
        assert!(!noop.iter().any(|e| matches!(e, Effect::Http(_))));
    }

    /// A filter fold rounds out defaults and commas for multi-selects.
    #[test]
    fn filter_to_wire_query_folds_defaults() {
        let f = CardBrowserFilter::default();
        let q = f.to_wire_query();
        assert_eq!(q.deck_id, "");
        assert_eq!(q.sort, "created_at");
        assert_eq!(q.page, 1);
        assert_eq!(q.per_page, 50);

        let f = CardBrowserFilter {
            deck_ids: vec![1, 2],
            flags: vec![1, 3],
            ..CardBrowserFilter::default()
        };
        let q = f.to_wire_query();
        assert_eq!(q.deck_id, "1,2");
        assert_eq!(q.flag, "1,3");
    }

    /// After a list refresh, `browser_selected` is re-synced to the freshest card
    /// with the same id (not left as a stale pre-action clone).
    #[test]
    fn cards_result_resyncs_selected_card() {
        let mut model = authenticated_model();
        // A stale selection: suspended=false.
        model.browser_selected = Some(browser_card(42));

        // The refreshed page contains the same card id but now suspended=true.
        let mut updated = browser_card(42);
        updated.suspended = Some(true);
        let response = ResponseBuilder::ok().body(browser_page(vec![updated])).build();
        let _ = update(Event::CardsResult(Ok(response)), &mut model);

        let sel = model.browser_selected.as_ref().expect("still selected");
        assert_eq!(sel.card_id, 42);
        assert_eq!(sel.suspended, Some(true), "selected card reflects the fresh data");
    }

    /// If the refreshed page no longer contains the selected card, the selection
    /// clears to `None`.
    #[test]
    fn cards_result_clears_selected_when_card_absent() {
        let mut model = authenticated_model();
        model.browser_selected = Some(browser_card(42));

        let response = ResponseBuilder::ok().body(browser_page(vec![browser_card(99)])).build();
        let _ = update(Event::CardsResult(Ok(response)), &mut model);

        assert!(model.browser_selected.is_none());
    }

    /// `BrowserFlagDone(Ok)` re-syncs the selection too (flag updates the list in
    /// place and must not leave `browser_selected.flag` stale).
    #[test]
    fn browser_flag_done_resyncs_selected_card() {
        let mut model = authenticated_model();
        model.browser_cards = vec![browser_card(42)];
        model.browser_selected = Some(browser_card(42));

        let response = ResponseBuilder::ok().body(flag_response(3)).build();
        let _ = update(Event::BrowserFlagDone(Ok(response)), &mut model);

        let sel = model.browser_selected.as_ref().expect("still selected");
        assert_eq!(sel.flag, Some(3), "selected card flag reflects the fresh data");
    }

    // --------------------------------------------------------------------
    // US-4.12 carve-out — note-type list for the browser filter dropdown
    // --------------------------------------------------------------------

    /// `NoteTypesRequested` emits `GET /note-types` with the bearer header.
    #[test]
    fn note_types_requested_emits_get_with_bearer() {
        let mut model = authenticated_model();
        let effects = update(Event::NoteTypesRequested, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(
                    req.operation.url.as_str(),
                    "http://127.0.0.1:3000/note-types"
                );
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A canned note-type list populates the model and is exposed (id + name) in
    /// the view.
    #[test]
    fn note_types_result_populates_view() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok()
            .body(vec![note_type(1, "Basic"), note_type(2, "Reversed")])
            .build();
        let _ = update(Event::NoteTypesResult(Ok(response)), &mut model);

        assert!(model.note_types_error.is_none());
        let vm = Anjuman.view(&model);
        assert_eq!(vm.note_types.len(), 2);
        assert_eq!(vm.note_types[0].id, 1);
        assert_eq!(vm.note_types[0].name, "Basic");
        assert_eq!(vm.note_types[1].name, "Reversed");
        assert_eq!(vm.note_types[0].field_names, vec!["Front".to_string(), "Back".to_string()]);
    }

    // --------------------------------------------------------------------
    // US-4.13 — notes browser
    // --------------------------------------------------------------------

    /// `NotesRequested` emits `GET /notes` with the folded query + bearer.
    #[test]
    fn notes_requested_emits_get_with_query() {
        let mut model = authenticated_model();
        let filter = NoteBrowserFilter {
            deck_ids: vec![5],
            note_type_ids: vec![1],
            q: Some("hola".to_string()),
            sort: "created_at".to_string(),
            page: 2,
            per_page: 25,
        };
        let effects = update(Event::NotesRequested { filter }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                let url = req.operation.url.to_string();
                assert!(url.starts_with("http://127.0.0.1:3000/notes?"), "url = {url}");
                assert!(url.contains("deck_id=5"), "url = {url}");
                assert!(url.contains("note_type_id=1"), "url = {url}");
                assert!(url.contains("q=hola"), "url = {url}");
                assert!(url.contains("page=2"), "url = {url}");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
        assert_eq!(model.notes_filter.as_ref().map(|f| f.deck_ids.clone()), Some(vec![5]));
    }

    /// `NotesResult(Ok)` populates the page fields and flattens fields + tags into
    /// the view.
    #[test]
    fn notes_result_populates_view() {
        let mut model = authenticated_model();
        let page = NoteBrowserPage {
            notes: vec![note(1), note(2)],
            page: 1,
            per_page: 50,
            total: 2,
        };
        let response = ResponseBuilder::ok().body(page).build();
        let _ = update(Event::NotesResult(Ok(response)), &mut model);

        assert_eq!(model.notes.len(), 2);
        assert_eq!(model.notes_total, 2);
        assert!(model.notes_error.is_none());

        let vm = Anjuman.view(&model);
        assert_eq!(vm.notes.len(), 2);
        assert_eq!(vm.notes_total, 2);
        assert_eq!(vm.notes[0].id, 1);
        assert_eq!(vm.notes[0].note_type_name, "Basic");
        assert_eq!(vm.notes[0].fields.get("Front").map(|s| s.as_str()), Some("hola"));
        assert_eq!(vm.notes[0].tags, vec!["spanish".to_string()]);
    }

    /// `NoteView::from(&NoteResponse)` carries the resolved sort field through so
    /// the shell can render it without resolving the note type's field itself.
    #[test]
    fn note_view_carries_sort_field() {
        let r = note(1);
        assert_eq!(r.sort_field.as_deref(), Some("hola"));

        let v = NoteView::from(&r);
        assert_eq!(v.sort_field.as_deref(), Some("hola"));

        // A missing sort field (no value to resolve) maps to `None`.
        let mut none = note(2);
        none.sort_field = None;
        assert_eq!(NoteView::from(&none).sort_field, None);
    }

    /// `NoteBrowserSelectNote` records the selection; an unknown id clears it.
    #[test]
    fn note_browser_select_note_records_selection() {
        let mut model = authenticated_model();
        model.notes = vec![note(1), note(2)];

        let _ = update(Event::NoteBrowserSelectNote { note_id: 2 }, &mut model);
        assert_eq!(model.notes_selected.as_ref().map(|n| n.id), Some(2));

        let _ = update(Event::NoteBrowserSelectNote { note_id: 999 }, &mut model);
        assert!(model.notes_selected.is_none());
    }

    /// A list refresh that no longer contains the selected note clears
    /// `notes_selected`.
    #[test]
    fn notes_selected_resyncs_after_list_refresh() {
        let mut model = authenticated_model();
        model.notes = vec![note(1), note(2)];
        model.notes_selected = Some(note(2));

        // The refreshed page contains only note 1 (not 2).
        let page = NoteBrowserPage {
            notes: vec![note(1)],
            page: 1,
            per_page: 50,
            total: 1,
        };
        let response = ResponseBuilder::ok().body(page).build();
        let _ = update(Event::NotesResult(Ok(response)), &mut model);

        assert!(model.notes_selected.is_none());
    }

    /// A list refresh that still contains the selected note re-selects it from the
    /// fresh data (not a stale clone).
    #[test]
    fn notes_selected_resyncs_to_fresh_data() {
        let mut model = authenticated_model();
        model.notes = vec![note(1)];
        model.notes_selected = Some(note(1));

        // Same note id, but with changed fields (e.g. an edit).
        let mut edited = note(1);
        edited.fields.insert("Front".to_string(), serde_json::Value::String("edited".to_string()));
        let page = NoteBrowserPage {
            notes: vec![edited],
            page: 1,
            per_page: 50,
            total: 1,
        };
        let response = ResponseBuilder::ok().body(page).build();
        let _ = update(Event::NotesResult(Ok(response)), &mut model);

        let sel = model.notes_selected.as_ref().expect("still selected");
        assert_eq!(sel.id, 1);
        assert_eq!(
            sel.fields.get("Front"),
            Some(&serde_json::Value::String("edited".to_string()))
        );
    }

    /// `NoteCreate` emits `POST /notes` with a `CreateNote` body over string fields.
    #[test]
    fn note_create_emits_post_with_body() {
        let mut model = authenticated_model();
        let effects = update(
            Event::NoteCreate {
                note_type_id: 1,
                deck_id: 9,
                fields: [("Front".to_string(), "hola".to_string())].into_iter().collect(),
            },
            &mut model,
        );

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "POST");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/notes");
                let body: CreateNote =
                    serde_json::from_slice(&req.operation.body).expect("body JSON");
                assert_eq!(body.note_type_id, 1);
                assert_eq!(body.deck_id, 9);
                assert_eq!(body.fields.get("Front"), Some(&serde_json::Value::String("hola".into())));
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `NoteSave` emits `PATCH /notes/{id}` with an `UpdateNote` body.
    #[test]
    fn note_save_emits_patch_with_body() {
        let mut model = authenticated_model();
        let effects = update(
            Event::NoteSave {
                note_id: 7,
                note_type_id: Some(2),
                fields: Some([("Back".to_string(), "hello".to_string())].into_iter().collect()),
            },
            &mut model,
        );

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "PATCH");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/notes/7");
                let body: UpdateNote =
                    serde_json::from_slice(&req.operation.body).expect("body JSON");
                assert_eq!(body.note_type_id, Some(2));
                assert!(body.fields.is_some());
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `NoteDelete` emits `DELETE /notes/{id}`.
    #[test]
    fn note_delete_emits_delete() {
        let mut model = authenticated_model();
        let effects = update(Event::NoteDelete { note_id: 7 }, &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "DELETE");
                assert_eq!(req.operation.url.as_str(), "http://127.0.0.1:3000/notes/7");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// A successful create clears the error and re-renders (the re-fetch is a
    /// chained `NotesRequested` command, not an effect here).
    #[test]
    fn note_created_clears_error_and_renders() {
        let mut model = authenticated_model();
        let response = ResponseBuilder::ok().body(note(3)).build();
        let effects = update(Event::NoteCreated(Ok(response)), &mut model);

        assert!(model.notes_error.is_none());
        assert!(effects.iter().any(|e| matches!(e, Effect::Render(_))));
    }
}
