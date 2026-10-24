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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- route_to_hash -----------------------------------------------------

    #[test]
    fn top_level_routes_have_no_params() {
        assert_eq!(route_to_hash(CurrentRoute::DeckList, None), "#/decks");
        assert_eq!(route_to_hash(CurrentRoute::Cards, None), "#/cards");
        assert_eq!(route_to_hash(CurrentRoute::Notes, Some(7)), "#/notes"); // id ignored
    }

    #[test]
    fn deck_panels_embed_the_deck_id() {
        assert_eq!(route_to_hash(CurrentRoute::Study, Some(42)), "#/decks/42/study");
        assert_eq!(route_to_hash(CurrentRoute::DeckOptions, Some(42)), "#/decks/42/options");
    }

    #[test]
    fn deck_panels_without_id_fall_back_to_deck_list() {
        assert_eq!(route_to_hash(CurrentRoute::Study, None), "#/decks");
        assert_eq!(route_to_hash(CurrentRoute::DeckOptions, None), "#/decks");
    }

    // ---- hash_to_event -----------------------------------------------------

    #[test]
    fn empty_and_root_hash_are_deck_list() {
        assert!(matches!(hash_to_event(""), Event::ShowDeckList));
        assert!(matches!(hash_to_event("#/"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("#"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("/"), Event::ShowDeckList));
    }

    #[test]
    fn plain_names_map_to_top_level_events() {
        assert!(matches!(hash_to_event("/decks"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("/cards"), Event::ShowCards));
        assert!(matches!(hash_to_event("/notes"), Event::ShowNotes));
    }

    #[test]
    fn hash_prefix_and_no_leading_slash_are_accepted() {
        assert!(matches!(hash_to_event("#/decks"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("decks"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("#cards"), Event::ShowCards));
    }

    #[test]
    fn deck_panel_hashes_map_to_open_events() {
        assert!(matches!(
            hash_to_event("#/decks/42/study"),
            Event::OpenDeck { deck_id: 42 }
        ));
        assert!(matches!(
            hash_to_event("#/decks/42/options"),
            Event::OpenDeckOptions { deck_id: 42 }
        ));
    }

    #[test]
    fn trailing_slash_is_ignored() {
        // The segment filter drops empty strings, so a trailing slash is benign.
        assert!(matches!(hash_to_event("#/decks/"), Event::ShowDeckList));
        assert!(matches!(
            hash_to_event("#/decks/42/study/"),
            Event::OpenDeck { deck_id: 42 }
        ));
    }

    #[test]
    fn malformed_deck_id_falls_back_to_deck_list() {
        assert!(matches!(
            hash_to_event("#/decks/not-a-number/study"),
            Event::ShowDeckList
        ));
    }

    #[test]
    fn unknown_route_falls_back_to_deck_list() {
        assert!(matches!(hash_to_event("#/settings"), Event::ShowDeckList));
        assert!(matches!(hash_to_event("#/decks/42/settings"), Event::ShowDeckList));
    }

    #[test]
    fn negative_or_zero_deck_id_is_still_openable_at_this_layer() {
        // The router only *parses*; whether an id is valid is the core's concern
        // (`OpenDeck` is a no-op for unknown ids). So `-1`/`0` parse through here.
        assert!(matches!(
            hash_to_event("#/decks/-1/study"),
            Event::OpenDeck { deck_id: -1 }
        ));
        assert!(matches!(
            hash_to_event("#/decks/0/options"),
            Event::OpenDeckOptions { deck_id: 0 }
        ));
    }

    // ---- round-trip --------------------------------------------------------

    #[test]
    fn route_to_hash_then_hash_to_event_is_identity() {
        let cases = [
            (CurrentRoute::DeckList, None),
            (CurrentRoute::Cards, None),
            (CurrentRoute::Notes, None),
            (CurrentRoute::Study, Some(9)),
            (CurrentRoute::DeckOptions, Some(9)),
        ];
        for (route, id) in cases {
            let hash = route_to_hash(route, id);
            let event = hash_to_event(&hash);
            let expected = match route {
                CurrentRoute::DeckList => Event::ShowDeckList,
                CurrentRoute::Cards => Event::ShowCards,
                CurrentRoute::Notes => Event::ShowNotes,
                CurrentRoute::Study => Event::OpenDeck { deck_id: 9 },
                CurrentRoute::DeckOptions => Event::OpenDeckOptions { deck_id: 9 },
            };
            // Events lack `PartialEq`; compare their Debug forms (struct + repr).
            assert_eq!(format!("{:?}", event), format!("{:?}", expected), "round-trip {hash}");
        }
    }
}
