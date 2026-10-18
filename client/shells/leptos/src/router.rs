//! Web-only URL routing for the Leptos shell.
//!
//! The *business* route is core-owned (`ViewModel.route` + `selected_deck`), but
//! the URL is inherently web-specific. This module makes the URL a **projection**
//! of the core route rather than a second source of truth:
//!
//! - `route_to_hash` mirrors the current core route into `location.hash`.
//! - `hash_to_event` parses a hash back into the *existing* core `Event`s
//!   (`OpenDeck`/`OpenDeckOptions`/`ShowCards`/`ShowNotes`/`ShowDeckList`), so a
//!   deep link or a `hashchange` (back/forward) is turned into an `Event` that the
//!   core re-asserts — never read directly during normal operation.
//!
//! Hash routing (`#/decks`, `#/decks/42/study`, …) is used deliberately: it needs
//! no server-side SPA fallback (a hard refresh always serves `index.html`), which
//! keeps this working under `trunk serve` and any static host without a history
//! rewrite rule.
//!
//! This is a presentation concern, so it stays shell-local (the `Event`/`ViewModel`
//! shapes are unchanged — the shell forwards core events, it doesn't invent any).

use shared::app::CurrentRoute;
use shared::Event;
use leptos::wasm_bindgen::JsCast;

/// The `location.hash` value for the given core route. `selected_deck_id` is only
/// needed for the two deck panels; other routes ignore it.
pub fn route_to_hash(route: CurrentRoute, selected_deck_id: Option<i64>) -> String {
    match route {
        CurrentRoute::DeckList => "#/decks".to_string(),
        CurrentRoute::Cards => "#/cards".to_string(),
        CurrentRoute::Notes => "#/notes".to_string(),
        CurrentRoute::Study => match selected_deck_id {
            Some(id) => format!("#/decks/{id}/study"),
            None => "#/decks".to_string(),
        },
        CurrentRoute::DeckOptions => match selected_deck_id {
            Some(id) => format!("#/decks/{id}/options"),
            None => "#/decks".to_string(),
        },
    }
}

/// Parse a `location.hash` (leading `#` stripped) back into a core `Event`, for
/// deep-link hydration and `hashchange` (back/forward) handling. Unknown/malformed
/// hashes map to the deck list.
pub fn hash_to_event(hash: &str) -> Event {
    // Strip the leading `#` and any leading `/`; split into segments.
    let path = hash.trim_start_matches('#').trim_start_matches('/');
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match segments.as_slice() {
        [] | ["decks"] => Event::ShowDeckList,
        ["cards"] => Event::ShowCards,
        ["notes"] => Event::ShowNotes,
        ["decks", id, "study"] => id
            .parse::<i64>()
            .map(|deck_id| Event::OpenDeck { deck_id })
            .unwrap_or(Event::ShowDeckList),
        ["decks", id, "options"] => id
            .parse::<i64>()
            .map(|deck_id| Event::OpenDeckOptions { deck_id })
            .unwrap_or(Event::ShowDeckList),
        _ => Event::ShowDeckList,
    }
}

/// Read the current `location.hash` (without the leading `#`).
pub fn current_hash() -> String {
    let Some(window) = web_sys::window() else {
        return String::new();
    };
    window.location().hash().unwrap_or_default()
}

/// Write `location.hash`. Uses `set_hash` (which pushes a history entry) so the
/// browser back/forward buttons walk the in-app URL history.
pub fn set_hash(hash: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let _ = window.location().set_hash(hash);
}

/// Register a `hashchange` listener that turns each URL change into `f(hash)`.
/// The listener lives for the whole app lifetime (a SPA mounts once), so the
/// closure is `mem::forget` (never dropped) rather than returned/stored — this
/// sidesteps the `Closure` type being `!Send`/`!Sync`, which can't be held in a
/// Leptos `StoredValue`.
pub fn on_hashchange(f: impl Fn(String) + 'static) {
    let closure = leptos::wasm_bindgen::prelude::Closure::wrap(Box::new(move |_ev: web_sys::Event| {
        f(current_hash());
    }) as Box<dyn Fn(web_sys::Event)>);

    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref());
    }

    std::mem::forget(closure);
}
