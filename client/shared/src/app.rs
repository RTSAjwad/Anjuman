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

use anjuman_contracts::auth::{LoginRequest, LoginResponse, UserResponse};
use anjuman_contracts::decks::{DeckCountsResponse, DeckResponse};
use anjuman_contracts::study::{StudyAdvance, StudyAdvanceBody, StudyCard, StudyCounts};

/// The base URL of the Anjuman server.
///
/// Uses `127.0.0.1` rather than `localhost` to avoid the browser resolving
/// `localhost` to IPv6 `::1` first (the server binds IPv4 `127.0.0.1` only),
/// which would fail the connection even after CORS is configured.
const API_URL: &str = "http://127.0.0.1:3000";

/// The key under which the JWT is persisted (by the shell's KV capability).
const TOKEN_KEY: &str = "auth_token";

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
    /// The card currently shown, if study has started and a card is due.
    pub current_card: Option<StudyCard>,
    /// Due counts for the deck being studied.
    pub counts: StudyCounts,
    /// A human-readable error from the last study request, if any.
    pub study_error: Option<String>,
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
    /// Begin studying the selected deck: fetch the first due card (+ counts).
    StartStudy,
    /// Answer the current card with a rating (1-4: Again/Hard/Good/Easy).
    Answer {
        rating: i32,
    },
    /// Leave the study screen and return to the deck list: clear the selection
    /// and reset in-flight study state (keeps auth + deck data intact).
    CloseDeck,

    // --- Core-local completion events (never cross the FFI boundary) ---

    #[serde(skip)]
    #[facet(skip)]
    StudyStarted(#[facet(opaque)] crux_http::Result<crux_http::Response<StudyAdvance>>),

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
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct ViewModel {
    pub email: String,
    pub authenticated: bool,
    /// The signed-in user's display name, when authenticated.
    pub display_name: String,
    pub busy: bool,
    pub error: Option<String>,
    /// Decks to render (title + per-state counts).
    pub decks: Vec<DeckSummary>,
    /// The selected deck (id + title + studyable) to show on the study screen,
    /// or `None` when showing the list.
    pub selected_deck: Option<DeckSummary>,
    /// A human-readable error from the last decks fetch, if any.
    pub decks_error: Option<String>,
    /// The card currently shown during study, or `None` when nothing is due
    /// (study not started, or the deck is finished).
    pub current_card: Option<StudyCardView>,
    /// Due counts for the deck being studied.
    pub counts: StudyCountsView,
    /// A human-readable error from the last study request, if any.
    pub study_error: Option<String>,
}

/// The shell-facing view of a card during study (a Facet/FFI-friendly mirror of
/// the wire `StudyCard`, following the same core-local mapping as `DeckSummary`
/// vs. `DeckResponse`). Bury/suspend and flags are out of scope for now.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct StudyCardView {
    pub card_id: i64,
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
            front: c.front.clone(),
            back: c.back.clone(),
            state: c.state.clone(),
            step_index: c.step_index,
            predicted_interval: c.predicted_interval.as_ref().map(|m| {
                m.iter()
                    .filter_map(|(k, v)| k.parse::<i32>().ok().map(|rating| (rating, *v)))
                    .collect()
            }),
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
                model.current_card = None;
                model.counts = StudyCounts::default();
                model.study_error = None;
                let clear = crux_kv::KeyValue::delete(TOKEN_KEY).then_send(Event::TokenDeleted);
                render::render().and(clear)
            }

            Event::OpenDeck { deck_id } => {
                // Only decks already in the list are openable; an unknown id is
                // a no-op (no crash, no bogus selection).
                if let Some(summary) = find_deck(&model.decks, deck_id) {
                    model.selected_deck = Some(summary);
                }
                render::render()
            }

            Event::CloseDeck => {
                // Back-navigation: drop the selection and any in-flight study
                // state, but keep auth + decks (unlike `Logout`).
                model.selected_deck = None;
                model.current_card = None;
                model.counts = StudyCounts::default();
                model.study_error = None;
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
                model.busy = false;
                model.study_error = None;
                render::render()
            }

            Event::StudyAdvanced(Err(e)) => {
                model.busy = false;
                model.study_error = Some(e.to_string());
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
                busy: model.busy,
                error: model.error.clone(),
                decks,
                selected_deck: model.selected_deck.clone(),
                decks_error: model.decks_error.clone(),
                current_card,
                counts,
                study_error: model.study_error.clone(),
            },
            Auth::Unauthenticated => ViewModel {
                email: String::new(),
                authenticated: false,
                display_name: String::new(),
                busy: model.busy,
                error: model.error.clone(),
                decks,
                selected_deck: None,
                decks_error: model.decks_error.clone(),
                current_card: None,
                counts: StudyCountsView::default(),
                study_error: None,
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
                },
            },
            ..Model::default()
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
    // US-4.5 — study session (single-card loop)
    // --------------------------------------------------------------------

    use anjuman_contracts::study::StudyAdvance;

    fn study_card(card_id: i64) -> StudyCard {
        StudyCard {
            card_id,
            note_id: 1,
            front: "front".to_string(),
            back: "back".to_string(),
            state: "new".to_string(),
            due_at: None,
            stability: 0.0,
            difficulty: 0.0,
            reps: 0,
            lapses: 0,
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
}
