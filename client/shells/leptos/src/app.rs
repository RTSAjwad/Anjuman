//! Leptos shell for the Anjuman core.
//!
//! This is the web front-end. It renders the core's `ViewModel` and forwards UI
//! events into `shared::Core<Anjuman>` via a reactive `Effect`. The shell is
//! pure glue: it consumes `ViewModel` and emits `Event`, never holding logic.
//!
//! UI is styled with Tailwind + daisyUI (see `src/ui.rs` for the thin wrappers
//! and `input.css`/`tailwind.css` for the generated stylesheet). Plain HTML
//! elements with DaisyUI classes are preferred for layout; `#[component]` wraps
//! the small set of reused primitives (button/badge/pagination).

use leptos::prelude::*;
use leptos::task;
use shared::app::{
    CardBrowserFilter, CardBrowserView, CurrentRoute, DeckOptionsEdit, DeckOptionsView,
    DeckSummary, DeckView, NoteBrowserFilter, NoteView, PresetSummary, StudyCardView,
    StudyCountsView,
};
use shared::{Event, ViewModel};
use std::collections::HashSet;

use crate::core_link;
use crate::router;
use crate::ui::{UiBadge, UiBadgeColor, UiButton, UiButtonAppearance, UiButtonSize, UiControlSize, UiInput, UiPagination, UiSelect, UiTab, UiTable, UiTabs, UiToggle};
use lepticons::{Icon, LucideGlyph};

/// Stable, app-lifetime state for the Browser's filter sidebar, card view, and
/// note view. Leptos `#[component]`s re-run their body whenever a prop signal
/// they read changes; the browser reads `vm` every render, so any state created
/// inside its body would be wiped on each ViewModel update (which happens after
/// every fetch/filter/action). Hoisting these `RwSignal`s here (created once in
/// `RootComponent`) keeps them stable for the app's lifetime.
///
/// Note: the card/note *view* is NOT here — it is the core-owned
/// `CurrentRoute::{Cards, Notes}` (`vm.route`), driven by `ShowCards`/`ShowNotes`.
#[derive(Clone, Copy)]
struct BrowserState {
    // --- Card-view filter sidebar ---
    deck: RwSignal<String>,
    note_type: RwSignal<String>,
    state: RwSignal<String>,
    flag: RwSignal<String>,
    sort: RwSignal<String>,
    query: RwSignal<String>,
    /// Current page (1-based).
    page: RwSignal<usize>,
    /// Guards the one-time "load first page + note types" fetch, so it only
    /// fires when the browser is first shown, not on every ViewModel-driven
    /// re-render (which would reset the user's active filter).
    initialized: RwSignal<bool>,
    // --- Note-view state ---
    /// Note-view filters (deck, note type, sort, search).
    note_deck_filter: RwSignal<String>,
    note_note_type: RwSignal<String>,
    note_sort: RwSignal<String>,
    note_query: RwSignal<String>,
    /// Note-view page (1-based).
    note_page: RwSignal<usize>,
    /// Guards first-fetch of the note list (fires once when Notes view opens).
    notes_initialized: RwSignal<bool>,
}

impl BrowserState {
    /// Reset all filter/fetch state to defaults. Called on logout so a fresh
    /// login starts from a clean slate (the `initialized` gate otherwise stays
    /// latched `true` from the previous session and never re-fires the fetch,
    /// leaving the browser table empty).
    fn reset(&self) {
        self.deck.set(String::new());
        self.note_type.set(String::new());
        self.state.set(String::new());
        self.flag.set(String::new());
        self.sort.set("created_at".to_string());
        self.query.set(String::new());
        self.page.set(1);
        self.initialized.set(false);
        self.note_deck_filter.set("0".to_string());
        self.note_note_type.set(String::new());
        self.note_sort.set("created_at".to_string());
        self.note_query.set(String::new());
        self.note_page.set(1);
        self.notes_initialized.set(false);
    }
}

#[component]
pub fn RootComponent() -> impl IntoView {
    let core = core_link::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::RestoreSession);

    // A place to hold the in-progress email/password while the user types (the
    // native `<input>` drives these signals via two-way `bind:value`).
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    Effect::new(move |_| {
        core_link::update(&core, event.get(), render);
    });

    let submit = move |_| {
        set_event.set(Event::LoginSubmit {
            email: email.get(),
            password: password.get(),
        });
    };
    // DEV ONLY: one-click login with seeded credentials, so the three roles can
    // be exercised without typing. Remove before any production use.
    let quick_login = move |creds: (&'static str, &'static str)| {
        set_event.set(Event::LoginSubmit {
            email: creds.0.to_string(),
            password: creds.1.to_string(),
        });
    };
    let logout = move |_| set_event.set(Event::Logout);

    // -----------------------------------------------------------------
    // Study-session UI state (created once, for the app's lifetime).
    //
    // `revealed` tracks whether the current card's answer is visible. It resets
    // to front-only whenever a different card is shown. The core now owns the
    // Study/Options sub-route (`ViewModel.selected_deck_view`) and auto-chains
    // `StartStudy`/`DeckOptionsRequested` on open, so the shell no longer fires
    // those itself.
    let revealed = RwSignal::new(false);
    let seen_card = StoredValue::new(0i64);

    Effect::new(move |_| {
        let vm = view.get();

        // Reset the reveal whenever the shown card changes.
        let card_id = vm.current_card.as_ref().map(|c| c.card_id).unwrap_or(0);
        if seen_card.get_value() != card_id {
            seen_card.set_value(card_id);
            revealed.set(false);
        }
    });

    // -----------------------------------------------------------------
    // Navigation. The core owns the route (`ViewModel.route` + `selected_deck` +
    // `selected_deck_view`); these closures just forward the matching events.

    // Back to the list. Shared by the sidebar and breadcrumb.
    let close_deck = move || set_event.set(Event::CloseDeck);

    // Open a deck and land on a sub-view (Study via the row title, Options via
    // the row's gear icon). `Callback` so it can cross the `#[component]`
    // boundary into `DeckList`.
    let navigate = Callback::new(move |(deck_id, view): (i64, DeckView)| match view {
        DeckView::Study => set_event.set(Event::OpenDeck { deck_id }),
        DeckView::Options => set_event.set(Event::OpenDeckOptions { deck_id }),
    });

    // Top-level navigation. These forward `Show*` events (the core owns the route
    // and, unlike the old shell-local `nav_selected`, top-level nav *always* wins
    // over an open deck by clearing it).
    let show_deck_list = move |_| set_event.set(Event::ShowDeckList);
    let show_cards = move |_| set_event.set(Event::ShowCards);

    // Whether the drawer sidebar is expanded (vs. icon-only). Mirrors the
    // `sidebar-toggle` checkbox so the toggle button's icon can swap between
    // open/close chevrons. Purely presentational — shell-local.
    let sidebar_expanded = RwSignal::new(false);

    // Browser state (card + note view), hoisted so it survives component
    // re-renders (see `BrowserState`). Created once for the app's lifetime.
    let browser_state = BrowserState {
        deck: RwSignal::new(String::new()),
        note_type: RwSignal::new(String::new()),
        state: RwSignal::new(String::new()),
        flag: RwSignal::new(String::new()),
        sort: RwSignal::new("created_at".to_string()),
        query: RwSignal::new(String::new()),
        page: RwSignal::new(1),
        initialized: RwSignal::new(false),
        note_deck_filter: RwSignal::new("0".to_string()),
        note_note_type: RwSignal::new(String::new()),
        note_sort: RwSignal::new("created_at".to_string()),
        note_query: RwSignal::new(String::new()),
        note_page: RwSignal::new(1),
        notes_initialized: RwSignal::new(false),
    };

    // Reset the card-browser state on logout so a subsequent login starts fresh.
    // `browser_state` is app-lifetime and its `initialized` gate latches `true`
    // after the first browse; without this reset, re-logging in leaves the table
    // empty (the fetch never re-fires). The route is reset by the core (`Logout`
    // sets `route = DeckList`), so the shell only needs to clear local UI state.
    Effect::new(move |_| {
        if !view.get().authenticated {
            browser_state.reset();
            revealed.set(false);
        }
    });

    // -----------------------------------------------------------------
    // URL routing. The core owns the route (`ViewModel.route`); the URL is a
    // web-only *projection* of it. We (a) mirror `route` into `location.hash`,
    // (b) deep-link/refresh by hydrating the initial hash once authenticated +
    // decks are loaded, and (c) turn back/forward (`hashchange`) into events.

    // Guards the mirror -> hashchange round-trip: when *we* write the hash, we set
    // this so the `hashchange` handler skips re-emitting the same event (which the
    // core would then re-process into the same route). User back/forward leaves it
    // clear, so those are forwarded as events.
    let suppress_hashchange = StoredValue::new(false);
    // Deep-link hydration fires exactly once (after auth + decks).
    let hydrated = StoredValue::new(false);

    // Mirror the core route into the URL. Uses `set_hash` (pushes a history entry)
    // only when the target differs from the current hash, so scrolling through
    // cards/notes/back produces a sensible back/forward history.
    //
    // Gated on `hydrated`: before auth + decks load we must NOT write the URL,
    // because that would clobber a deep-link hash (e.g. `#/decks/42/study`) with
    // `#/decks` before the hydration effect has read it.
    //
    // IMPORTANT: `view.get()` is read *before* the `hydrated` early-return. If an
    // effect `return`s before reading any reactive signal, it registers no
    // dependencies and silently never re-runs — which would leave the URL stuck at
    // its initial value forever. Reading `view` up front keeps the subscription
    // alive across logout -> login -> route changes (the effect re-runs on every
    // `view` change and, once `hydrated` is latched, starts mirroring).
    Effect::new(move |_| {
        let vm = view.get();
        if !hydrated.get_value() {
            return;
        }
        let target = router::route_to_hash(vm.route, vm.selected_deck.as_ref().map(|d| d.id));
        let current = router::current_hash();
        if current != target {
            suppress_hashchange.set_value(true);
            router::set_hash(&target);
        }
    });

    // Register the `hashchange` listener once. It maps URL changes that were *not*
    // caused by our own mirror write back into core events (browser back/forward,
    // or the user editing the address bar's hash). `router::on_hashchange` forgets
    // the closure on purpose (SPA lifetime), so we only guard against registering
    // twice with a plain bool.
    let hashchange_registered = StoredValue::new(false);
    {
        let set_event = set_event;
        Effect::new(move |_| {
            if hashchange_registered.get_value() {
                return;
            }
            hashchange_registered.set_value(true);
            let set_event = set_event;
            router::on_hashchange(move |hash| {
                if suppress_hashchange.get_value() {
                    suppress_hashchange.set_value(false);
                    return;
                }
                set_event.set(router::hash_to_event(&hash));
            });
        });
    }

    // Deep-link / refresh hydration: once authenticated and decks are loaded,
    // parse the initial hash once and fire the matching event. Deferred because
    // `OpenDeck { id }` is a no-op until the deck list contains that id.
    {
        let set_event = set_event;
        Effect::new(move |_| {
            if hydrated.get_value() {
                return;
            }
            let vm = view.get();
            if !vm.authenticated || vm.decks.is_empty() {
                return;
            }
            let hash = router::current_hash();
            hydrated.set_value(true);
            if hash.is_empty() {
                // No deep link: normalise to the canonical deck-list hash so the
                // mirror's next run (and back/forward) starts from a known state.
                suppress_hashchange.set_value(true);
                router::set_hash("#/decks");
            } else {
                set_event.set(router::hash_to_event(&hash));
            }
        });
    }

    view! {
        {move || {
                let vm: ViewModel = view.get();
                if vm.authenticated {
                    let display_name = vm.display_name.clone();
                    let route = vm.route;
                    let can_manage_notes = vm.permissions.can_manage_notes;
                    view! {
                        <div class="drawer lg:drawer-open">
                            <input
                                id="sidebar-toggle"
                                type="checkbox"
                                class="drawer-toggle"
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    sidebar_expanded.set(checked);
                                }
                            />

                            <div class="drawer-content flex h-svh flex-col">
                                <nav class="navbar min-h-12 w-full gap-2 bg-base-300 px-3">
                                    <label
                                        for="sidebar-toggle"
                                        aria-label="toggle sidebar"
                                        class="btn btn-square btn-ghost drawer-button"
                                    >
                                        {move || {
                                            if sidebar_expanded.get() {
                                                view! { <Icon glyph=LucideGlyph::PanelRightClose size="20" /> }.into_any()
                                            } else {
                                                view! { <Icon glyph=LucideGlyph::PanelRightOpen size="20" /> }.into_any()
                                            }
                                        }}
                                    </label>

                                    {match route {
                                        CurrentRoute::DeckList => view! {
                                            <div class="breadcrumbs">
                                                <ul>
                                                    <li><span aria-current="page">"Decks"</span></li>
                                                </ul>
                                            </div>
                                        }
                                        .into_any(),
                                        CurrentRoute::Cards | CurrentRoute::Notes => view! {
                                            <UiTabs>
                                                <UiTab
                                                    active=Signal::derive(move || route == CurrentRoute::Cards)
                                                    on_click=show_cards.clone()
                                                >
                                                    "Cards"
                                                </UiTab>
                                                {if can_manage_notes {
                                                    view! {
                                                        <UiTab
                                                            active=Signal::derive(move || route == CurrentRoute::Notes)
                                                            on_click=move |_| set_event.set(Event::ShowNotes)
                                                        >
                                                            "Notes"
                                                        </UiTab>
                                                    }
                                                    .into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </UiTabs>
                                        }
                                        .into_any(),
                                        CurrentRoute::Study => {
                                            let counts = vm.counts.clone();
                                            let current_card = vm.current_card.clone();
                                            let close_deck = close_deck.clone();
                                            let deck_breadcrumb = vm
                                                .selected_deck
                                                .as_ref()
                                                .map(|d| deck_ancestor_titles(&vm.decks, d.id))
                                                .unwrap_or_default();
                                            view! {
                                                <div class="breadcrumbs flex-1">
                                                    <ul>
                                                        <li><button on:click=move |_| close_deck()>"Decks"</button></li>
                                                        {deck_breadcrumb.into_iter().map(|title| view! {
                                                            <li>{title}</li>
                                                        }).collect_view()}
                                                        <li><span aria-current="page">"Study"</span></li>
                                                    </ul>
                                                </div>
                                                {study_count_badges(counts)}
                                                {if let Some(card) = current_card {
                                                    let flag_signal = Signal::derive(move || {
                                                        view.get().current_card.as_ref().map(|c| c.flag).unwrap_or(0)
                                                    });
                                                    view! {
                                                        <StudyControls card=card set_event=set_event />
                                                        <FlagDropdown
                                                            flag=flag_signal
                                                            on_select=move |f| {
                                                                set_event.set(Event::FlagCard { flag: f as i32 });
                                                            }
                                                        />
                                                    }
                                                    .into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            }
                                            .into_any()
                                        }
                                        CurrentRoute::DeckOptions => {
                                            let close_deck = close_deck.clone();
                                            let deck_breadcrumb = vm
                                                .selected_deck
                                                .as_ref()
                                                .map(|d| deck_ancestor_titles(&vm.decks, d.id))
                                                .unwrap_or_default();
                                            view! {
                                                <div class="breadcrumbs flex-1">
                                                    <ul>
                                                        <li><button on:click=move |_| close_deck()>"Decks"</button></li>
                                                        {deck_breadcrumb.into_iter().map(|title| view! {
                                                            <li>{title}</li>
                                                        }).collect_view()}
                                                        <li><span aria-current="page">"Options"</span></li>
                                                    </ul>
                                                </div>
                                            }
                                            .into_any()
                                        }
                                    }}
                                </nav>

                                <main class="flex min-h-0 flex-1 flex-col overflow-hidden">
                                    {match route {
                                        CurrentRoute::DeckList => view! {
                                            <div class="h-full overflow-y-auto p-4">
                                                <DeckList vm=view.into() on_navigate=navigate />
                                            </div>
                                        }
                                        .into_any(),
                                        CurrentRoute::Cards | CurrentRoute::Notes => view! {
                                            <Browser vm=view.into() state=browser_state set_event=set_event />
                                        }
                                        .into_any(),
                                        CurrentRoute::Study | CurrentRoute::DeckOptions => {
                                            if vm.selected_deck.is_some() {
                                                match route {
                                                    CurrentRoute::Study => view! {
                                                        <StudyScreen vm=vm.clone() revealed=revealed set_event=set_event />
                                                    }
                                                    .into_any(),
                                                    CurrentRoute::DeckOptions => view! {
                                                        <div class="h-full overflow-y-auto">
                                                            <DeckOptionsPanel
                                                                deck_id=vm.selected_deck.as_ref().map(|d| d.id).unwrap_or(0)
                                                                options=vm.deck_options.clone()
                                                                error=vm.deck_options_error.clone()
                                                                presets=vm.presets.clone()
                                                                presets_error=vm.presets_error.clone()
                                                                can_manage_presets=vm.permissions.can_manage_presets
                                                                set_event=set_event />
                                                        </div>
                                                    }
                                                    .into_any(),
                                                    _ => ().into_any(),
                                                }
                                            } else {
                                                view! {
                                                    <div class="h-full overflow-y-auto p-4">
                                                        <DeckList vm=view.into() on_navigate=navigate />
                                                    </div>
                                                }
                                                .into_any()
                                            }
                                        }
                                    }}
                                </main>
                            </div>

                            <div class="drawer-side lg:is-drawer-close:overflow-visible">
                                <label for="sidebar-toggle" aria-label="close sidebar" class="drawer-overlay"></label>
                                <div class="app-sidebar flex min-h-full w-64 flex-col items-start bg-base-200 is-drawer-close:w-14 is-drawer-open:w-64">
                                    <div class="app-sidebar__brand w-full is-drawer-close:hidden">"Anjuman"</div>
                                    <ul class="menu w-full grow">
                                        <li>
                                            <button
                                                class:menu-active=move || route == CurrentRoute::DeckList
                                                class="is-drawer-close:tooltip is-drawer-close:tooltip-right"
                                                data-tip="Decks"
                                                on:click=show_deck_list
                                            >
                                                <Icon glyph=LucideGlyph::Library size="16" class="my-1.5 inline-block" />
                                                <span class="is-drawer-close:hidden">"Decks"</span>
                                            </button>
                                        </li>
                                        <li>
                                            <button
                                                class:menu-active=move || route == CurrentRoute::Cards || route == CurrentRoute::Notes
                                                class="is-drawer-close:tooltip is-drawer-close:tooltip-right"
                                                data-tip="Browser"
                                                on:click=show_cards.clone()
                                            >
                                                <Icon glyph=LucideGlyph::Table size="16" class="my-1.5 inline-block" />
                                                <span class="is-drawer-close:hidden">"Browser"</span>
                                            </button>
                                        </li>
                                    </ul>
                                    <div class="mt-auto w-full">
                                        <p class="app-sidebar__user px-3 pt-3 is-drawer-close:hidden">{display_name.clone()}</p>
                                        <ul class="menu w-full">
                                            <li>
                                                <button
                                                    class="is-drawer-close:tooltip is-drawer-close:tooltip-right"
                                                    data-tip="Log out"
                                                    on:click=logout
                                                >
                                                    <Icon glyph=LucideGlyph::LogOut size="16" class="my-1.5 inline-block" />
                                                    <span class="is-drawer-close:hidden">"Log out"</span>
                                                </button>
                                            </li>
                                        </ul>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <main class="app-login">
                            <h1>"Anjuman"</h1>
                            <div style="display: flex; gap: 0.5rem; flex-direction: column;">
                                <UiInput value=email placeholder="Email" />
                                <UiInput value=password placeholder="Password" input_type="password" />
                                <UiButton
                                    appearance=UiButtonAppearance::Primary
                                    on_click=submit
                                    disabled=vm.busy
                                >
                                    "Log in"
                                </UiButton>
                                {vm.error.clone().map(|err| view! {
                                    <p style="color: #c00;">{err}</p>
                                })}
                                <div style="display: flex; gap: 0.5rem;">
                                    {["admin", "teacher", "student"].into_iter().map(|role| {
                                        let creds: (&str, &str) = match role {
                                            "admin" => ("admin@school1.com", "admin123"),
                                            "teacher" => ("teacher@school1.com", "teach123"),
                                            "student" => ("student@school1.com", "stud123"),
                                            _ => unreachable!(),
                                        };
                                        let q = quick_login.clone();
                                        view! {
                                            <UiButton
                                                appearance=UiButtonAppearance::Subtle
                                                on_click=move |_| q(creds)
                                                disabled=vm.busy
                                            >
                                                {role}
                                            </UiButton>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        </main>
                    }
                    .into_any()
                }
            }}
    }
}

/// Renders the authenticated user's deck list (title + due counts), with
/// explicit error and empty states. Pure rendering of the `ViewModel` — no
/// logic beyond deciding what the three states look like.
///
/// Takes `vm` as a reactive `Signal` (not a plain value) so the component
/// re-renders when the core writes new counts into `decks` after study — a plain
/// owned `ViewModel` prop would be snapshotted at first render and never update.
#[component]
fn deck_list(vm: Signal<ViewModel>, on_navigate: Callback<(i64, DeckView), ()>) -> impl IntoView {
    let vm = vm.get();
    // Error state — a deck fetch failed.
    if let Some(err) = vm.decks_error {
        return view! {
            <div class="card bg-base-100 shadow-sm">
                <p style="color: #c00;">"Could not load decks: " {err}</p>
            </div>
        }
        .into_any();
    }

    // Empty state — signed in but no decks yet.
    if vm.decks.is_empty() {
        return view! {
            <div class="card bg-base-100 shadow-sm">
                <p style="color: #888;">"No decks yet. Create one to get started."</p>
            </div>
        }
        .into_any();
    }

    // The deck list itself: a collapsible tree (title + due counts). The core
    // builds the tree (`DeckSummary.children`); the shell only renders it and
    // holds which nodes are expanded (transient UI state — see the divergence
    // note in docs/support/screens.md). Collapsed by default.
    //
    // We render the tree with plain elements rather than thaw's `Tree`/
    // `TreeItem`: thaw indents the whole row (badges included) by depth and
    // mounts its stylesheet at runtime, which made the due-count columns drift
    // between parent/child rows. A hand-rolled grid keeps the badge columns
    // perfectly aligned regardless of depth.
    let decks = vm.decks;
    let open_items = RwSignal::new(HashSet::new());
    view! {
        <div class="card bg-base-100 shadow-sm">
            <div class="thaw-deck-header" role="row">
                <span class="thaw-deck-col">"Deck"</span>
                <span class="thaw-deck-col">"New"</span>
                <span class="thaw-deck-col">"Learn"</span>
                <span class="thaw-deck-col">"Due"</span>
            </div>
            <div class="thaw-deck-tree" role="tree">
                {deck_subtree(decks, 0, open_items, on_navigate)}
            </div>
        </div>
    }
    .into_any()
}

/// Compute the ancestor chain (root → target, inclusive) of a deck within the
/// nested deck tree, as a list of titles. Used to render full breadcrumb paths
/// for a selected sub-deck (e.g. `Parent > Child > Study`). Returns the empty
/// path when `deck_id` isn't found (the caller falls back to the selected
/// deck's own title, so a missing path is harmless).
fn deck_ancestor_titles(decks: &[DeckSummary], deck_id: i64) -> Vec<String> {
    fn walk(decks: &[DeckSummary], deck_id: i64, path: &mut Vec<String>) -> bool {
        for deck in decks {
            path.push(deck.title.clone());
            if deck.id == deck_id {
                return true;
            }
            if walk(&deck.children, deck_id, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    let mut path = Vec::new();
    walk(decks, deck_id, &mut path);
    path
}

/// The card browser (US-4.12): a management-oriented card table with
/// filter/sort controls (a hand-rolled collapsible filter tree in the sidebar),
/// pagination, and per-card Suspend/Bury/Reschedule/Flag actions. Consumes the
/// core's `browser_*` `ViewModel` fields and forwards the
/// `Browser*`/`CardsRequested` events.
#[component]
fn browser(
    vm: Signal<ViewModel>,
    state: BrowserState,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let vm_sig = vm;

    // Request the first page on entry (no filter, default sort/page), plus the
    // note-type list for the filter sidebar. Guarded by `initialized` so it only
    // fires the first time the browser is shown — not on every ViewModel-driven
    // re-render, which would clobber the user's active filter with an unfiltered
    // refetch.
    let initialized = state.initialized;
    Effect::new(move |_| {
        if initialized.get() {
            return;
        }
        initialized.set(true);
        set_event.set(Event::NoteTypesRequested);
        // The event bridge carries one event at a time; a second `set_event.set`
        // in the same tick would clobber the note-types request above. Defer the
        // cards fetch so both reach the core.
        let set_event = set_event;
        task::spawn_local(async move {
            set_event.set(Event::CardsRequested {
                filter: CardBrowserFilter {
                    deck_ids: Vec::new(),
                    note_type_ids: Vec::new(),
                    q: None,
                    states: Vec::new(),
                    flags: Vec::new(),
                    sort: "created_at".to_string(),
                    page: 1,
                    per_page: 50,
                },
            });
        });
    });

    // Filter/sort state lives in the hoisted `BrowserState` (stable across
    // re-renders); these are the individual signals backing the sidebar.
    let deck_value = state.deck;
    let note_type_value = state.note_type;
    let state_value = state.state;
    let flag_value = state.flag;
    let sort_value = state.sort;
    let q_value = state.query;
    let page = state.page;

    // The deck tree (hierarchy preserved for the sidebar) and the note-type list
    // are read reactively inside the render closure below (never snapshotted here).

    // Build a filter from the current signal state and fire `CardsRequested`.
    // `page` defaults to 1 (a filter change resets to the first page).
    let with_filter = move |page: i64| -> CardBrowserFilter {
        let mut deck_ids = Vec::new();
        if let Ok(d) = deck_value.get().parse::<i64>() {
            if d != 0 {
                deck_ids.push(d);
            }
        }
        let mut note_type_ids = Vec::new();
        if let Ok(n) = note_type_value.get().parse::<i64>() {
            if n != 0 {
                note_type_ids.push(n);
            }
        }
        let mut states = Vec::new();
        if !state_value.get().is_empty() {
            states.push(state_value.get());
        }
        let mut flags = Vec::new();
        if let Ok(f) = flag_value.get().parse::<i64>() {
            flags.push(f);
        }
        CardBrowserFilter {
            deck_ids,
            note_type_ids,
            q: if q_value.get().trim().is_empty() {
                None
            } else {
                Some(q_value.get().trim().to_string())
            },
            states,
            flags,
            sort: sort_value.get(),
            page,
            per_page: 50,
        }
    };

    // A no-arg variant for the tree leaves (which don't receive a useful event).
    let fire_filter = move || {
        page.set(1);
        let filter = with_filter(1);
        set_event.set(Event::CardsRequested { filter });
    };

    // Watch the pagination signal (thaw's `Pagination` mutates it directly) and
    // fire a fetch when the page changes away from what the core last returned.
    // Self-syncing (no explicit guard): after the fetch, `browser_page` catches up
    // to `page`, so the effect stops firing.
    Effect::new(move |_| {
        let p = page.get();
        let current = vm_sig.get().browser_page.max(1) as usize;
        if p != current {
            let filter = with_filter(p as i64);
            set_event.set(Event::CardsRequested { filter });
        }
    });

    // Suspend/bury/flag are read-only derived signals over the live
    // `browser_selected`. The custom `CardSwitch` mirrors state from these (never
    // writes locally) and emits the directional event on click.
    let selected_suspend = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.suspended).unwrap_or(false)
    });
    let selected_bury = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.buried).unwrap_or(false)
    });
    let selected_flag = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.flag.to_string()).unwrap_or_default()
    });

    // Selecting a leaf sets its category's filter value and re-applies; clicking
    // the already-selected leaf de-selects it (resets that category's filter).
    let select_deck = {
        let fire_filter = fire_filter.clone();
        move |id: String| {
            if deck_value.get() == id {
                deck_value.set(String::new());
            } else {
                deck_value.set(id);
            }
            fire_filter();
        }
    };
    let select_note_type = {
        let fire_filter = fire_filter.clone();
        move |id: String| {
            if note_type_value.get() == id {
                note_type_value.set(String::new());
            } else {
                note_type_value.set(id);
            }
            fire_filter();
        }
    };
    let select_state = {
        let fire_filter = fire_filter.clone();
        move |s: String| {
            if state_value.get() == s {
                state_value.set(String::new());
            } else {
                state_value.set(s);
            }
            fire_filter();
        }
    };
    let select_flag = {
        let fire_filter = fire_filter.clone();
        move |f: String| {
            if flag_value.get() == f {
                flag_value.set(String::new());
            } else {
                flag_value.set(f);
            }
            fire_filter();
        }
    };
    let select_sort = {
        let fire_filter = fire_filter.clone();
        move |s: String| {
            // Sort always defaults to a value; clicking the active one resets it.
            if sort_value.get() == s {
                sort_value.set("created_at".to_string());
            } else {
                sort_value.set(s);
            }
            fire_filter();
        }
    };

    // Clone the select closures for use inside the `view!` moves.
    let deck_sel = select_deck;
    let note_sel = select_note_type;
    let state_sel = select_state;
    let flag_sel = select_flag;
    let sort_sel = select_sort;

    // Active selections (reactive — `RwSignal`, so `--active` updates live).
    let deck_active = deck_value.clone();
    let note_active = note_type_value.clone();
    let state_active = state_value.clone();
    let flag_active = flag_value.clone();
    let sort_active = sort_value.clone();

    // Search fires on Enter (the form's submit). `fire_filter` resets the page
    // and applies the current filter (including the query).
    let search_apply = fire_filter.clone();

    // -----------------------------------------------------------------
    // Note view. Mirrors the card view: a filter sidebar (deck/note-type/sort),
    // a table, a right-hand inspector pane, and bottom pagination. Drives the
    // notes core surface (`NotesRequested { filter }`, `notes_page/total/per_page`,
    // `NoteCreate`/`NoteSave`/`NoteDelete`).

    let note_deck_filter = state.note_deck_filter;
    let note_note_type = state.note_note_type;
    let note_sort = state.note_sort;
    let note_query = state.note_query;
    let note_page = state.note_page;
    let notes_initialized = state.notes_initialized;

    // Lazy first fetch of the note list when the Notes view is toggled on.
    Effect::new(move |_| {
        if vm_sig.get().route != CurrentRoute::Notes {
            return;
        }
        if notes_initialized.get() {
            return;
        }
        notes_initialized.set(true);
        set_event.set(Event::NotesRequested {
            filter: NoteBrowserFilter::default(),
        });
    });

    let with_note_filter = move |page: i64| -> NoteBrowserFilter {
        let mut deck_ids = Vec::new();
        if let Ok(d) = note_deck_filter.get().parse::<i64>() {
            if d != 0 {
                deck_ids.push(d);
            }
        }
        let mut note_type_ids = Vec::new();
        if let Ok(n) = note_note_type.get().parse::<i64>() {
            if n != 0 {
                note_type_ids.push(n);
            }
        }
        NoteBrowserFilter {
            deck_ids,
            note_type_ids,
            q: if note_query.get().trim().is_empty() {
                None
            } else {
                Some(note_query.get().trim().to_string())
            },
            sort: note_sort.get(),
            page,
            per_page: 50,
        }
    };

    let fire_note_filter = move |page: i64| {
        note_page.set(page.max(1) as usize);
        set_event.set(Event::NotesRequested { filter: with_note_filter(page) });
    };

    // Selecting a note-view filter leaf sets the value and re-applies; clicking
    // the active leaf de-selects it.
    let select_note_deck = {
        let fire_note_filter = fire_note_filter.clone();
        move |id: String| {
            if note_deck_filter.get() == id {
                note_deck_filter.set("0".to_string());
            } else {
                note_deck_filter.set(id);
            }
            fire_note_filter(1);
        }
    };
    let select_note_type_filter = {
        let fire_note_filter = fire_note_filter.clone();
        move |id: String| {
            if note_note_type.get() == id {
                note_note_type.set(String::new());
            } else {
                note_note_type.set(id);
            }
            fire_note_filter(1);
        }
    };
    let select_note_sort = {
        let fire_note_filter = fire_note_filter.clone();
        move |s: String| {
            if note_sort.get() == s {
                note_sort.set("created_at".to_string());
            } else {
                note_sort.set(s);
            }
            fire_note_filter(1);
        }
    };

    // Watch the note pagination signal and fetch a new page when it changes.
    Effect::new(move |_| {
        if vm_sig.get().route != CurrentRoute::Notes {
            return;
        }
        let p = note_page.get();
        let current = vm_sig.get().notes_page.max(1) as usize;
        if p != current {
            set_event.set(Event::NotesRequested { filter: with_note_filter(p as i64) });
        }
    });

    // Stable derived signals read by the render (created once in the component
    // body — creating a `Signal::derive` inside the per-render closure would leak
    // a new memo every render).
    let notes_selected_signal = Signal::derive(move || vm_sig.get().notes_selected.clone());
    let browser_selected_signal = Signal::derive(move || vm_sig.get().browser_selected.clone());

    // Pagination page-count derives (created once in the component body).
    let browser_page_count = Signal::derive(move || {
        let total = vm_sig.get().browser_total.max(0) as usize;
        let per = vm_sig.get().browser_per_page.max(1) as usize;
        ((total + per - 1) / per).max(1)
    });
    let notes_page_count = Signal::derive(move || {
        let total = vm_sig.get().notes_total.max(0) as usize;
        let per = vm_sig.get().notes_per_page.max(1) as usize;
        ((total + per - 1) / per).max(1)
    });

    // Owned snapshot of the note list is re-read reactively inside the render
    // closure below (never captured once here).

    view! {
        <div class="card-browser">
            <div class="card-browser__body">
            <aside class="card-browser__sidebar">
                {match vm_sig.get().route {
                    CurrentRoute::Cards => view! {
                        <form
                            class="card-browser__sidebar-search"
                            on:submit=move |e: leptos::ev::SubmitEvent| {
                                e.prevent_default();
                                search_apply();
                            }
                        >
                            <div class="join w-full">
                                <UiInput value=q_value placeholder="Search…" class="w-full join-item" />
                                <button type="submit" aria-label="Search" class="btn btn-square join-item">
                                    <Icon glyph=LucideGlyph::Search size="16" />
                                </button>
                            </div>
                        </form>

                        {move || vm_sig.get().browser_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load cards: " {err}</p>
                        })}
                        {move || vm_sig.get().note_types_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load note types: " {err}</p>
                        })}

                        <ul class="menu card-browser__menu">
                            {filter_branch(
                                "Decks",
                                deck_filter_tree(
                                    vm_sig.get().decks.clone(),
                                    deck_active,
                                    deck_sel,
                                ),
                            )}

                            {filter_branch(
                                "Note types",
                                {move || vm_sig.get().note_types.iter().map(|nt| {
                                    let id_str = nt.id.to_string();
                                    let active = note_active.get() == id_str;
                                    let name = nt.name.clone();
                                    let sel = note_sel.clone();
                                    filter_leaf(active, id_str, name, move |id| sel(id))
                                }).collect_view()},
                            )}

                            {filter_branch(
                                "State",
                                ["new", "learning", "review", "relearning"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = state_active.get() == s_val;
                                    let sel = state_sel.clone();
                                    let label: &'static str = match s {
                                        "new" => "New",
                                        "learning" => "Learning",
                                        "review" => "Review",
                                        "relearning" => "Relearning",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}

                            {filter_branch(
                                "Flag",
                                ["1", "2", "3", "4", "5", "6", "7"].into_iter().map(|f| {
                                    let f_val = f.to_string();
                                    let active = flag_active.get() == f_val;
                                    let sel = flag_sel.clone();
                                    let color = flag_color(f.parse::<i64>().unwrap());
                                    let label = flag_label(f.parse::<i64>().unwrap());
                                    view! {
                                        <li>
                                            <button class:menu-active=active on:click=move |_| sel(f_val.clone())>
                                                <span class="card-browser__flag-sw">
                                                    <span class="card-browser__flag-swatch" style=format!("background-color: {color}")></span>
                                                    <span>{label}</span>
                                                </span>
                                            </button>
                                        </li>
                                    }
                                }).collect_view(),
                            )}

                            {filter_branch(
                                "Sort",
                                ["created_at", "due_at", "deck", "question"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = sort_active.get() == s_val;
                                    let sel = sort_sel.clone();
                                    let label: &'static str = match s {
                                        "created_at" => "Created",
                                        "due_at" => "Due",
                                        "deck" => "Deck",
                                        "question" => "Question",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}
                        </ul>
                    }
                    .into_any(),
                    CurrentRoute::Notes => view! {
                        <form
                            class="card-browser__sidebar-search"
                            on:submit=move |e: leptos::ev::SubmitEvent| {
                                e.prevent_default();
                                fire_note_filter(1);
                            }
                        >
                            <div class="join w-full">
                                <UiInput value=note_query placeholder="Search…" class="w-full join-item" />
                                <button type="submit" aria-label="Search" class="btn btn-square join-item">
                                    <Icon glyph=LucideGlyph::Search size="16" />
                                </button>
                            </div>
                        </form>

                        {move || vm_sig.get().notes_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load notes: " {err}</p>
                        })}
                        {move || vm_sig.get().note_types_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load note types: " {err}</p>
                        })}

                        <ul class="menu card-browser__menu">
                            {filter_branch(
                                "Decks",
                                deck_filter_tree(
                                    vm_sig.get().decks.clone(),
                                    note_deck_filter,
                                    select_note_deck,
                                ),
                            )}

                            {filter_branch(
                                "Note types",
                                {move || vm_sig.get().note_types.iter().map(|nt| {
                                    let id_str = nt.id.to_string();
                                    let active = note_note_type.get() == id_str;
                                    let name = nt.name.clone();
                                    let sel = select_note_type_filter.clone();
                                    filter_leaf(active, id_str, name, move |id| sel(id))
                                }).collect_view()},
                            )}

                            {filter_branch(
                                "Sort",
                                ["created_at", "note_type", "deck", "question"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = note_sort.get() == s_val;
                                    let sel = select_note_sort.clone();
                                    let label: &'static str = match s {
                                        "created_at" => "Created",
                                        "note_type" => "Note type",
                                        "deck" => "Deck",
                                        "question" => "Question",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}
                        </ul>
                    }
                    .into_any(),
                    _ => ().into_any(),
                }}
            </aside>

            <div class="card-browser__main">
                {match vm_sig.get().route {
                    CurrentRoute::Cards => view! {
                        <div class="card-browser__content">
                            <UiTable>
                                <thead>
                                    <tr>
                                        <th>"Card"</th>
                                        <th>"Deck"</th>
                                        <th>"State"</th>
                                        <th>"Flag"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || vm_sig.get().browser_cards.iter().map(|c| {
                                        let card_id = c.card_id;
                                        let is_selected = vm_sig.get().browser_selected.as_ref().map(|c| c.card_id) == Some(card_id);
                                        let front = c.front.clone();
                                        let deck_title = c.deck_title.clone();
                                        let state = c.state.clone();
                                        let flag = c.flag;
                                        let suspended = c.suspended;
                                        let buried = c.buried;
                                        view! {
                                            <tr
                                                class:card-browser__row--selected=is_selected
                                                on:click=move |_| set_event.set(Event::BrowserSelectCard { card_id })
                                            >
                                                <td>
                                                    <span class="card-browser__front">
                                                        {front}
                                                        {if suspended { " (suspended)" } else { "" }}
                                                        {if buried { " (buried)" } else { "" }}
                                                    </span>
                                                </td>
                                                <td>{deck_title}</td>
                                                <td>{state}</td>
                                                <td>{flag_swatch(flag)}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </UiTable>

                            <div class="card-browser__pagination">
                                <span>{move || vm_sig.get().browser_total.to_string()}{" cards"}</span>
                                <UiPagination page=page page_count=browser_page_count />
                            </div>
                        </div>

                        <CardInspector
                            card=browser_selected_signal
                            suspend=selected_suspend
                            bury=selected_bury
                            flag=selected_flag
                            set_event=set_event
                        />
                    }
                    .into_any(),
                    CurrentRoute::Notes => view! {
                        <div class="card-browser__content">
                            <UiTable>
                                <thead>
                                    <tr>
                                        <th>"Sort field"</th>
                                        <th>"Note type"</th>
                                        <th>"Tags"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || vm_sig.get().notes.iter().map(|n| {
                                        let note_id = n.id;
                                        let note_type_name = n.note_type_name.clone();
                                        let sort_field = n.sort_field.clone().unwrap_or_default();
                                        let tags = n.tags.join(", ");
                                        view! {
                                            <tr
                                                class:card-browser__row--selected=move || vm_sig.get().notes_selected.as_ref().map(|n| n.id) == Some(note_id)
                                                on:click=move |_| set_event.set(Event::NoteBrowserSelectNote { note_id })
                                            >
                                                <td>
                                                    <span class="card-browser__front">{sort_field}</span>
                                                </td>
                                                <td>{note_type_name}</td>
                                                <td>{tags}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </UiTable>

                            <div class="card-browser__pagination">
                                <span>{move || vm_sig.get().notes_total.to_string()}{" notes"}</span>
                                <UiPagination page=note_page page_count=notes_page_count />
                            </div>
                        </div>

                        <NoteInspector
                            note=notes_selected_signal
                        />
                    }
                    .into_any(),
                    _ => ().into_any(),
                }}
            </div>
        </div>
        </div>
    }
}

/// The card-view information/action pane (right-hand side of the browser).
/// Always rendered; shows the selected card's fields plus the per-card actions
/// (Suspend / Bury / Reschedule / Flag), or a hint when nothing is selected.
#[component]
fn CardInspector(
    card: Signal<Option<CardBrowserView>>,
    suspend: Signal<bool>,
    bury: Signal<bool>,
    flag: Signal<String>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let reschedule_days = RwSignal::new(String::new());

    view! {
        <aside class="card-browser__inspector">
            <h3 class="card-browser__inspector-title">"Selected card"</h3>

            {if let Some(card) = card.get() {
                let card_id = card.card_id;
                view! {
                    <dl class="card-browser__inspector-fields">
                        <div class="card-browser__inspector-row">
                            <dt>"Card"</dt>
                            <dd>{card.card_id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note"</dt>
                            <dd>{card.note_id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Deck"</dt>
                            <dd>{card.deck_title}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note type"</dt>
                            <dd>{card.note_type_name}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"State"</dt>
                            <dd>{card.state}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Suspended"</dt>
                            <dd>
                                <CardSwitch
                                    checked=suspend
                                    label=""
                                    on_toggle=move || {
                                        if suspend.get() {
                                            set_event.set(Event::BrowserUnsuspendCard { card_id });
                                        } else {
                                            set_event.set(Event::BrowserSuspendCard { card_id });
                                        }
                                    }
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Buried"</dt>
                            <dd>
                                <CardSwitch
                                    checked=bury
                                    label=""
                                    disabled=suspend
                                    on_toggle=move || {
                                        if bury.get() {
                                            set_event.set(Event::BrowserUnburyCard { card_id });
                                        } else {
                                            set_event.set(Event::BrowserBuryCard { card_id });
                                        }
                                    }
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Flag"</dt>
                            <dd>
                                <FlagDropdown
                                    flag=Signal::derive(move || flag.get().parse::<i64>().unwrap_or(0))
                                    on_select=move |f| {
                                        set_event.set(Event::BrowserFlagCard { card_id, flag: f as i32 });
                                    }
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Reschedule"</dt>
                            <dd>
                                <div class="card-browser__inspector-reschedule">
                                    <div class="join">
                                        <UiInput value=reschedule_days input_type="number" placeholder="days" class="join-item" />
                                        <UiButton
                                            appearance=UiButtonAppearance::Subtle
                                            square=true
                                            join_item=true
                                            aria_label="Reschedule"
                                            on_click=move |_| {
                                                if let Ok(d) = reschedule_days.get().trim().parse::<i64>() {
                                                    set_event.set(Event::BrowserRescheduleCard { card_id, days: d });
                                                    reschedule_days.set(String::new());
                                                }
                                            }
                                        >
                                            <Icon glyph=LucideGlyph::Check size="16" />
                                        </UiButton>
                                    </div>
                                </div>
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                            <dt>"Front"</dt>
                            <dd class="card-browser__inspector-text">{card.front}</dd>
                        </div>
                        <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                            <dt>"Back"</dt>
                            <dd class="card-browser__inspector-text">{card.back}</dd>
                        </div>
                    </dl>
                }
                .into_any()
            } else {
                view! {
                    <span class="card-browser__hint">"Select a card to view and manage it."</span>
                }
                .into_any()
            }}
        </aside>
    }
}

/// The note-view information pane (mirrors `CardInspector`). Read-only: shows
/// the selected note's fields, note type, and tags. Note editing/creation/delete
/// are out of scope for now, so there are no inputs or actions here.
#[component]
fn NoteInspector(
    note: Signal<Option<NoteView>>,
) -> impl IntoView {
    view! {
        <aside class="card-browser__inspector">
            <h3 class="card-browser__inspector-title">"Note"</h3>

            {if let Some(note) = note.get() {
                let note_type_name = note.note_type_name.clone();
                let tags = note.tags.clone();
                let mut fields: Vec<(String, String)> = note.fields.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                view! {
                    <dl class="card-browser__inspector-fields">
                        <div class="card-browser__inspector-row">
                            <dt>"ID"</dt>
                            <dd>{note.id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note type"</dt>
                            <dd>{note_type_name}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Tags"</dt>
                            <dd>{tags.join(", ")}</dd>
                        </div>
                        {fields.iter().map(|(name, value)| {
                            let name = name.clone();
                            let value = value.clone();
                            view! {
                                <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                                    <dt>{name}</dt>
                                    <dd class="card-browser__inspector-text">{value}</dd>
                                </div>
                            }
                        }).collect_view()}
                    </dl>
                }
                .into_any()
            } else {
                view! {
                    <span class="card-browser__hint">"Select a note to view it."</span>
                }
                .into_any()
            }}
        </aside>
    }
}

/// An unidirectional suspend/bury toggle. This is **purely reflective**: it has
/// no writable state of its own — the checked state comes only from the `checked`
/// signal (derived from the core's `ViewModel`), and a click emits a *directional*
/// event via `on_toggle` (suspend vs unsuspend, bury vs unbury). The core's
/// response re-syncs `checked`, which drives the visible toggle back to truth.
///
/// The toggle itself is the owned `UiToggle` primitive (signal-driven
/// `prop:checked`, no DOM mirroring).
#[component]
fn CardSwitch(
    checked: Signal<bool>,
    label: &'static str,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <label class="card-browser__switch">
            <UiToggle checked=checked on_toggle=on_toggle disabled=disabled />
            <span class="card-browser__switch-label">{label}</span>
        </label>
    }
}

/// A collapsible filter category in the browser sidebar, rendered as a daisyUI
/// `menu` submenu (`<details>`/`<summary>`). Expansion is native (no signal
/// needed — the browser tracks the `<details>` `open` state), which is exactly
/// what the "collapsible submenu" pattern wants. A plain `fn` (not
/// `#[component]`) so it sits inline in the reactive `view!` and re-renders when
/// its data (e.g. the async-loaded note types) arrives.
fn filter_branch(label: &'static str, leaves: impl IntoView) -> impl IntoView {
    view! {
        <li>
            <details>
                <summary>{label}</summary>
                <ul>{leaves}</ul>
            </details>
        </li>
    }
}

/// A single selectable filter leaf, rendered as a daisyUI `menu` item (native
/// `<button>`), highlighted with `menu-active` when selected.
fn filter_leaf(
    active: bool,
    value: String,
    label: String,
    on_select: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <li>
            <button class:menu-active=active on:click=move |_| on_select(value.clone())>
                {label}
            </button>
        </li>
    }
}

/// Renders the deck hierarchy as nested daisyUI `menu` submenus. A deck with
/// children is an expandable `<details>`/`<summary>` (the `<summary>` is plain
/// text, so daisyUI's chevron renders exactly like its docs). Because the
/// summary toggles the submenu (it can't also be a filter), the parent deck's
/// own selectable entry is rendered as the **first item inside** its submenu
/// (labelled "this deck only"), followed by its children. A leaf deck is a
/// plain selectable `<li><button>`.
fn deck_filter_tree(
    decks: Vec<DeckSummary>,
    active: RwSignal<String>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_filter_node(deck, active.clone(), on_select.clone()))
        .collect_view()
}

/// A single deck node in the filter menu.
fn deck_filter_node(
    deck: DeckSummary,
    active: RwSignal<String>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    let id_str = deck.id.to_string();
    let title = deck.title;
    let has_children = !deck.children.is_empty();
    let children = deck.children;

    // Active-state reader for this deck (used by the selectable entries).
    let is_active = {
        let id_str = id_str.clone();
        move || active.get() == id_str
    };

    if has_children {
        let select_self = on_select.clone();
        let self_id = id_str.clone();
        // The parent's own "filter to this deck only" entry, shown first inside
        // the expanded submenu (the summary itself only toggles the submenu).
        let self_item = view! {
            <li>
                <button class:menu-active=is_active.clone() on:click=move |_| select_self(self_id.clone())>
                    {title.clone()}
                    <span class="card-browser__deck-self">{"this deck only"}</span>
                </button>
            </li>
        };
        view! {
            <li>
                <details>
                    <summary>{title}</summary>
                    <ul>
                        {self_item}
                        {deck_filter_tree(children, active.clone(), on_select)}
                    </ul>
                </details>
            </li>
        }
        .into_any()
    } else {
        view! {
            <li>
                <button class:menu-active=is_active on:click=move |_| on_select(id_str.clone())>
                    {title}
                </button>
            </li>
        }
        .into_any()
    }
}

/// Anki flag colour name for a 0-7 value (0 = none).
fn flag_label(flag: i64) -> &'static str {
    match flag {
        0 => "—",
        1 => "Red",
        2 => "Orange",
        3 => "Green",
        4 => "Blue",
        5 => "Pink",
        6 => "Turquoise",
        7 => "Purple",
        _ => "—",
    }
}

/// CSS colour for a 0-7 Anki flag (0 = no colour). Uses CSS named colours.
fn flag_color(flag: i64) -> &'static str {
    match flag {
        1 => "red",
        2 => "orange",
        3 => "green",
        4 => "blue",
        5 => "pink",
        6 => "turquoise",
        7 => "purple",
        _ => "transparent",
    }
}

/// A small coloured dot + label for a flag value (used where free HTML is
/// rendered, e.g. the filter sidebar, the table, and the inspector `<dd>`).
fn flag_swatch(flag: i64) -> impl IntoView {
    let label = flag_label(flag);
    let color = flag_color(flag);
    view! {
        <span class="card-browser__flag-sw">
            <span
                class="card-browser__flag-swatch"
                style=format!("background-color: {color}")
            ></span>
            <span>{label}</span>
        </span>
    }
}

/// A dropdown flag picker: a square ghost button with a coloured Lucide `Flag`
/// icon as the trigger, and a list of the eight flag values (0 = none, 1–7 the
/// Anki colours). Clicking an item forwards `on_select(value)` and closes the
/// menu. The trigger icon is tinted with the current flag's colour (neutral when
/// unset).
///
/// Open/close is driven entirely by a local `RwSignal` (no native CSS `popover`
/// and no DaisyUI `.dropdown`/`.menu` classes), so it is deterministic and
/// immune to global resets. The menu is absolutely positioned below the trigger
/// via a `dropdown dropdown-end` wrapper and DaisyUI's `.dropdown-content`, with
/// the self-contained `.flag-dropdown__menu` class supplying the list layout.
#[component]
fn FlagDropdown(
    flag: Signal<i64>,
    on_select: impl Fn(i64) + Clone + 'static,
) -> impl IntoView {
    // The trigger stroke: the current flag's colour, or a neutral grey when no
    // flag is set (flag 0 maps to "transparent", which would hide the icon).
    let stroke = move || {
        let f = flag.get();
        if f == 0 {
            "currentColor".to_string()
        } else {
            flag_color(f).to_string()
        }
    };

    let open = RwSignal::new(false);
    let toggle = move |_| open.update(|o| *o = !*o);

    let choose = move |f: i64| {
        open.set(false);
        on_select(f);
    };

    view! {
        <div class="flag-dropdown">
            <div tabindex="0" role="button" class="btn btn-ghost btn-square" aria-label="Set flag" on:click=toggle>
                <Icon glyph=LucideGlyph::Flag size="20" stroke=stroke />
            </div>
            <ul
                class="flag-dropdown__menu rounded-box bg-base-100 shadow-sm"
                style:display=move || if open.get() { "flex" } else { "none" }
            >
                <li>
                    <button class:menu-active=move || flag.get() == 0 on:click={
                        let choose = choose.clone();
                        move |_| choose(0)
                    }>
                        {flag_swatch(0)}
                    </button>
                </li>
                {[1i64, 2, 3, 4, 5, 6, 7].into_iter().map(|f| {
                    let choose = choose.clone();
                    view! {
                        <li>
                            <button class:menu-active=move || flag.get() == f on:click=move |_| choose(f)>
                                {flag_swatch(f)}
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

/// The study screen: one card at a time, centred in the available space, with
/// the reveal/answer controls pinned to the bottom. Due counts live in the
/// header (see `study_count_badges`); the deck title is in the breadcrumb. Pure
/// rendering of the `ViewModel` snapshot — study state (`StartStudy` gating,
/// reveal reset) lives in `RootComponent`.
#[component]
fn study_screen(
    vm: ViewModel,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let current = vm.current_card;
    let study_error = vm.study_error;
    let card_action_error = vm.card_action_error;
    let busy = vm.busy;

    view! {
        <div class="study-screen">
            {study_error.map(|err| view! {
                <p style="color: #c00;">{err}</p>
            })}
            {card_action_error.map(|err| view! {
                <p style="color: #c00;">{err}</p>
            })}

            <div class="study-screen__body">
                {match current {
                    Some(card) => study_card_view(card, revealed, set_event).into_any(),
                    None => {
                        if busy {
                            view! {
                                <p style="color: #888;">"Loading…"</p>
                            }
                            .into_any()
                        } else {
                            study_done_view(set_event).into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}

/// The due-count badges shown in the header, next to the breadcrumb.
fn study_count_badges(counts: StudyCountsView) -> impl IntoView {
    view! {
        <div class="thaw-study-counts">
            <UiBadge color=UiBadgeColor::Brand>
                {counts.new_count.to_string()}
            </UiBadge>
            <UiBadge color=UiBadgeColor::Danger>
                {(counts.learning_count + counts.relearning_count).to_string()}
            </UiBadge>
            <UiBadge color=UiBadgeColor::Success>
                {counts.review_count.to_string()}
            </UiBadge>
        </div>
    }
}

/// Study-card actions shown in the top bar while a card is in hand: Bury,
/// Suspend (card + note), and Reschedule (set due date). Each forwards a core
/// `Event` using the current card; the core owns the HTTP + state. Flagging is a
/// separate `FlagDropdown` rendered next to this, mirroring the card browser
/// inspector.
#[component]
fn StudyControls(card: StudyCardView, set_event: WriteSignal<Event>) -> impl IntoView {
    // Reschedule: a day offset the user types, then confirms with a check.
    let days = RwSignal::new(String::new());
    let reschedule = move |_| {
        if let Ok(d) = days.get().trim().parse::<i64>() {
            set_event.set(Event::RescheduleCard { days: d });
            days.set(String::new());
        }
    };

    let buried = card.buried;
    let suspended = card.suspended;

    view! {
        <div class="study-actions">
            {if buried {
                view! {
                    <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::UnburyCard)>"Unbury"</UiButton>
                }.into_any()
            } else {
                view! {
                    <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::BuryCard)>"Bury"</UiButton>
                }.into_any()
            }}
            <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::BuryNote)>"Bury note"</UiButton>
            {if suspended {
                view! {
                    <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::UnsuspendCard)>"Unsuspend"</UiButton>
                }.into_any()
            } else {
                view! {
                    <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::SuspendCard)>"Suspend"</UiButton>
                }.into_any()
            }}
            <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small on_click=move |_| set_event.set(Event::SuspendNote)>"Suspend note"</UiButton>

            <div class="join">
                <UiInput value=days input_type="number" size=UiControlSize::Small placeholder="days" class="join-item w-20" />
                <UiButton appearance=UiButtonAppearance::Subtle size=UiButtonSize::Small join_item=true aria_label="Reschedule" on_click=reschedule>
                    <Icon glyph=LucideGlyph::Check size="20" />
                </UiButton>
            </div>
        </div>
    }
}

/// Editable deck-options form (US-4.10): renders the selected deck's preset as
/// controls (number/text inputs, bool switches, enum selectors), applies local
/// validation, and forwards `Event::DeckOptionsSave` with the edited values.
///
/// Nothing is shown until the preset has been fetched (via the Options route's
/// `DeckOptionsRequested`). The form state is local (this component) and reset
/// on each entry to Options.
#[component]
fn deck_options_panel(
    deck_id: i64,
    options: Option<DeckOptionsView>,
    error: Option<String>,
    presets: Vec<PresetSummary>,
    presets_error: Option<String>,
    can_manage_presets: bool,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let options = match options {
        Some(o) => o,
        None => {
            // Error and empty states for an unresolved preset.
            return view! {
                <section class="deck-options">
                    {if let Some(err) = error {
                        view! { <p class="deck-options__error">"Could not load deck options: " {err}</p> }
                            .into_any()
                    } else {
                        view! { <p class="deck-options__empty">"No deck options to show."</p> }
                            .into_any()
                    }}
                </section>
            }
            .into_any();
        }
    };

    let name = options.name.clone();
    // The system-owned global default (id 0) is read-only — see US-4.10 and the
    // server report.
    let read_only = options.id == 0;
    // Editing/saving is gated on the management permission (US-9.5); without it
    // the preset is shown read-only (view stays, controls are locked).
    let can_edit = can_manage_presets && !read_only;
    // Editable draft state, initialized from the loaded preset. Rebuilt on each
    // entry to Options (see `RootComponent`, which mounts this only on the
    // Options route).
    let state = DeckOptionsEditState::from_view(&options);
    // Local (shell-side) save error: parse errors that never reach the core.
    // `error` above carries the core-set fetch/save error; this is separate.
    let local_error = RwSignal::new(None::<String>);

    // US-4.11: request the school's preset list once, on entry to the screen, so
    // the assign picker can render it (the server's `GET /deck-options` does not
    // include the system default id 0, so the shell synthesizes that entry).
    Effect::new(move |_| {
        set_event.set(Event::DeckOptionsListRequested);
    });

    // The assign picker: a native `<select>` showing the deck's current preset
    // id. On a user change, forward `Event::DeckOptionsAssign` directly (no thaw
    // `Select`/`Effect` workaround needed — the `on:change` only fires on a real
    // user change). The core re-fetches after assignment, so `options.id` then
    // matches the newly-assigned id.
    let assign_selection = RwSignal::new(options.id.to_string());

    let on_save = move |_| {
        local_error.set(None);
        match state.to_edit() {
            Ok(edit) => set_event.set(Event::DeckOptionsSave(edit)),
            Err(msg) => local_error.set(Some(msg)),
        }
    };

    view! {
        <section class="deck-options" class:deck-options--readonly=move || !can_edit>
            <header class="deck-options__header">
                <h2 class="deck-options__title">"Deck options"</h2>
                <span class="deck-options__preset">
                    {name}
                    {if read_only { " — read-only" } else { "" }}
                </span>
            </header>

            {if read_only {
                view! {
                    <p class="deck-options__readonly-notice">
                        "This is the shared default preset. It can't be edited — create your own preset to customise scheduling."
                    </p>
                }
                .into_any()
            } else if !can_manage_presets {
                view! {
                    <p class="deck-options__readonly-notice">
                        "You don't have permission to change deck options."
                    </p>
                }
                .into_any()
            } else {
                ().into_any()
            }}

            {if can_manage_presets {
                view! {
                    <div class="deck-options-assign">
                        <span class="deck-options-assign__label">"Preset"</span>
                        <select
                            class="select select-bordered"
                            prop:value=move || assign_selection.get()
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                assign_selection.set(value.clone());
                                if let Ok(id) = value.parse::<i64>() {
                                    set_event.set(Event::DeckOptionsAssign {
                                        deck_id,
                                        options_id: id,
                                    });
                                }
                            }
                        >
                            <option value="0">"Default"</option>
                            {presets.iter().map(|p| {
                                let id = p.id.to_string();
                                let name = p.name.clone();
                                view! { <option value=id>{name}</option> }
                            }).collect_view()}
                        </select>
                        {presets_error.map(|err| view! {
                            <span class="deck-options__error">{err}</span>
                        })}
                    </div>
                }
                .into_any()
            } else {
                ().into_any()
            }}

            <OptionsGroup title="Daily limits">
                <NumberField label="New cards/day" value=state.new_per_day placeholder="20" />
                <NumberField label="Maximum reviews/day" value=state.review_per_day placeholder="200" />
            </OptionsGroup>

            <OptionsGroup title="New cards">
                <TextField label="Learning steps" value=state.learning_steps placeholder="1m 10m" />
                <SelectField
                    label="Insertion order"
                    value=state.insertion_order
                    options=INSERTION_ORDER_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Lapses">
                <TextField label="Relearning steps" value=state.relearning_steps placeholder="10m" />
                <NumberField label="Leech threshold" value=state.leech_threshold placeholder="8" />
                <SelectField label="Leech action" value=state.leech_action options=LEECH_ACTION_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Display order">
                <SelectField label="New card gather order" value=state.new_gather_order options=NEW_GATHER_OPTIONS />
                <SelectField label="New card sort order" value=state.new_sort_order options=NEW_SORT_OPTIONS />
                <SelectField label="New/review order" value=state.new_review_order options=NEW_REVIEW_OPTIONS />
                <SelectField label="Interday learning/review order" value=state.interday_order options=NEW_REVIEW_OPTIONS />
                <SelectField label="Review sort order" value=state.review_sort_order options=REVIEW_SORT_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Burying">
                <ToggleField label="Bury new siblings" checked=state.bury_new />
                <ToggleField label="Bury review siblings" checked=state.bury_review />
                <ToggleField label="Bury interday learning siblings" checked=state.bury_interday />
            </OptionsGroup>

            <OptionsGroup title="Audio">
                <ToggleField label="Don't play audio automatically" checked=state.dont_play_audio_automatically />
                <ToggleField label="Skip question when replaying answer" checked=state.skip_question_when_replaying_answer />
            </OptionsGroup>

            <OptionsGroup title="Timers">
                <NumberField label="Maximum answer seconds" value=state.maximum_answer_seconds placeholder="60" />
                <ToggleField label="Show on-screen timer" checked=state.show_on_screen_timer />
                <ToggleField label="Stop on-screen timer on answer" checked=state.stop_timer_on_answer />
            </OptionsGroup>

            <OptionsGroup title="Auto advance">
                <NumberField label="Seconds to show question" value=state.auto_advance_show_question placeholder="0" />
                <NumberField label="Seconds to show answer" value=state.auto_advance_show_answer placeholder="0" />
                <ToggleField label="Wait for audio" checked=state.auto_advance_wait_for_audio />
                <SelectField label="Question action" value=state.auto_advance_question_action options=QUESTION_ACTION_OPTIONS />
                <SelectField label="Answer action" value=state.auto_advance_answer_action options=ANSWER_ACTION_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="FSRS">
                <NumberField label="Desired retention (%) " value=state.desired_retention placeholder="90" />
                <EasyDaysField days=state.easy_days />
            </OptionsGroup>

            <OptionsGroup title="Advanced">
                <NumberField label="Maximum interval (days)" value=state.maximum_interval placeholder="36500" />
                <FieldRow label="FSRS parameters">
                    <span class="deck-options__readonly">
                        {if options.fsrs_parameters.is_empty() {
                            "Default".to_string()
                        } else {
                            format_parameters(&options.fsrs_parameters)
                        }}
                    </span>
                </FieldRow>
            </OptionsGroup>

            <footer class="deck-options__footer">
                {move || {
                    if let Some(msg) = local_error.get() {
                        view! { <p class="deck-options__error">{msg}</p> }.into_any()
                    } else if let Some(msg) = error.clone() {
                        view! { <p class="deck-options__error">{msg}</p> }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
                {if can_edit {
                    view! {
                        <UiButton appearance=UiButtonAppearance::Primary on_click=on_save>
                            "Save"
                        </UiButton>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }}
            </footer>
        </section>
    }
    .into_any()
}

/// The editable draft state for a deck-options preset. Each control binds to a
/// `RwSignal`; `to_edit` parses the raw strings into a typed `DeckOptionsEdit`
/// (or returns a human-readable parse error). Enums stay as their `snake_case`
/// wire form; numbers/steps are parsed on save.
#[derive(Clone, Copy)]
struct DeckOptionsEditState {
    new_per_day: RwSignal<String>,
    review_per_day: RwSignal<String>,
    leech_threshold: RwSignal<String>,
    maximum_answer_seconds: RwSignal<String>,
    maximum_interval: RwSignal<String>,
    desired_retention: RwSignal<String>,
    auto_advance_show_question: RwSignal<String>,
    auto_advance_show_answer: RwSignal<String>,
    learning_steps: RwSignal<String>,
    relearning_steps: RwSignal<String>,
    bury_new: RwSignal<bool>,
    bury_review: RwSignal<bool>,
    bury_interday: RwSignal<bool>,
    dont_play_audio_automatically: RwSignal<bool>,
    skip_question_when_replaying_answer: RwSignal<bool>,
    show_on_screen_timer: RwSignal<bool>,
    stop_timer_on_answer: RwSignal<bool>,
    auto_advance_wait_for_audio: RwSignal<bool>,
    new_gather_order: RwSignal<String>,
    new_sort_order: RwSignal<String>,
    new_review_order: RwSignal<String>,
    interday_order: RwSignal<String>,
    review_sort_order: RwSignal<String>,
    insertion_order: RwSignal<String>,
    leech_action: RwSignal<String>,
    auto_advance_question_action: RwSignal<String>,
    auto_advance_answer_action: RwSignal<String>,
    easy_days: [RwSignal<String>; 7],
}

impl DeckOptionsEditState {
    /// Initialise the draft from a loaded preset's current values.
    fn from_view(o: &DeckOptionsView) -> Self {
        DeckOptionsEditState {
            new_per_day: RwSignal::new(o.new_per_day.to_string()),
            review_per_day: RwSignal::new(o.review_per_day.to_string()),
            leech_threshold: RwSignal::new(o.leech_threshold.to_string()),
            maximum_answer_seconds: RwSignal::new(o.maximum_answer_seconds.to_string()),
            maximum_interval: RwSignal::new(o.maximum_interval.to_string()),
            desired_retention: RwSignal::new(format_desired_retention(o.desired_retention)),
            auto_advance_show_question: RwSignal::new(format_float_input(o.auto_advance_seconds_show_question)),
            auto_advance_show_answer: RwSignal::new(format_float_input(o.auto_advance_seconds_show_answer)),
            learning_steps: RwSignal::new(format_steps(&o.learning_steps)),
            relearning_steps: RwSignal::new(format_steps(&o.relearning_steps)),
            bury_new: RwSignal::new(o.bury_new),
            bury_review: RwSignal::new(o.bury_review),
            bury_interday: RwSignal::new(o.bury_interday),
            dont_play_audio_automatically: RwSignal::new(o.dont_play_audio_automatically),
            skip_question_when_replaying_answer: RwSignal::new(o.skip_question_when_replaying_answer),
            show_on_screen_timer: RwSignal::new(o.show_on_screen_timer),
            stop_timer_on_answer: RwSignal::new(o.stop_timer_on_answer),
            auto_advance_wait_for_audio: RwSignal::new(o.auto_advance_wait_for_audio),
            new_gather_order: RwSignal::new(o.new_gather_order.clone()),
            new_sort_order: RwSignal::new(o.new_sort_order.clone()),
            new_review_order: RwSignal::new(o.new_review_order.clone()),
            interday_order: RwSignal::new(o.interday_order.clone()),
            review_sort_order: RwSignal::new(o.review_sort_order.clone()),
            insertion_order: RwSignal::new(o.insertion_order.clone()),
            leech_action: RwSignal::new(o.leech_action.clone()),
            auto_advance_question_action: RwSignal::new(o.auto_advance_question_action.clone()),
            auto_advance_answer_action: RwSignal::new(o.auto_advance_answer_action.clone()),
            easy_days: {
                // Always 7 entries (the wire array is Monday-first); defensively
                // default any missing entry to "normal".
                let arr: [RwSignal<String>; 7] =
                    std::array::from_fn(|_| RwSignal::new("normal".to_string()));
                for (i, d) in o.easy_days.iter().enumerate().take(7) {
                    arr[i].set(d.clone());
                }
                arr
            },
        }
    }

    /// Parse the raw draft into a typed `DeckOptionsEdit`. All fields are `Some`
    /// (the whole preset is sent on save); enums stay snake_case; numbers and
    /// steps are parsed, returning `Err` with a readable message on failure.
    fn to_edit(&self) -> Result<DeckOptionsEdit, String> {
        let int = |s: &RwSignal<String>, what: &str| -> Result<i64, String> {
            let t = s.get().trim().to_string();
            t.parse::<i64>().map_err(|_| format!("{what} must be a whole number"))
        };
        let float = |s: &RwSignal<String>, what: &str| -> Result<f64, String> {
            let t = s.get().trim().to_string();
            t.parse::<f64>().map_err(|_| format!("{what} must be a number"))
        };

        Ok(DeckOptionsEdit {
            new_per_day: Some(int(&self.new_per_day, "New cards/day")?),
            review_per_day: Some(int(&self.review_per_day, "Maximum reviews/day")?),
            leech_threshold: Some(int(&self.leech_threshold, "Leech threshold")?),
            maximum_answer_seconds: Some(int(&self.maximum_answer_seconds, "Maximum answer seconds")?),
            maximum_interval: Some(int(&self.maximum_interval, "Maximum interval")?),
            // Desired retention is entered as a percent (e.g. 90); convert to 0..1.
            desired_retention: Some(float(&self.desired_retention, "Desired retention")? / 100.0),
            auto_advance_seconds_show_question: Some(float(&self.auto_advance_show_question, "Auto-advance seconds (question)")?),
            auto_advance_seconds_show_answer: Some(float(&self.auto_advance_show_answer, "Auto-advance seconds (answer)")?),
            learning_steps: Some(self.learning_steps.get().trim().to_string()),
            relearning_steps: Some(self.relearning_steps.get().trim().to_string()),
            bury_new: Some(self.bury_new.get()),
            bury_review: Some(self.bury_review.get()),
            bury_interday: Some(self.bury_interday.get()),
            dont_play_audio_automatically: Some(self.dont_play_audio_automatically.get()),
            skip_question_when_replaying_answer: Some(self.skip_question_when_replaying_answer.get()),
            show_on_screen_timer: Some(self.show_on_screen_timer.get()),
            stop_timer_on_answer: Some(self.stop_timer_on_answer.get()),
            auto_advance_wait_for_audio: Some(self.auto_advance_wait_for_audio.get()),
            new_gather_order: Some(self.new_gather_order.get()),
            new_sort_order: Some(self.new_sort_order.get()),
            new_review_order: Some(self.new_review_order.get()),
            interday_order: Some(self.interday_order.get()),
            review_sort_order: Some(self.review_sort_order.get()),
            insertion_order: Some(self.insertion_order.get()),
            leech_action: Some(self.leech_action.get()),
            auto_advance_question_action: Some(self.auto_advance_question_action.get()),
            auto_advance_answer_action: Some(self.auto_advance_answer_action.get()),
            easy_days: Some(self.easy_days.iter().map(|d| d.get()).collect()),
            // Name and parameters are not edited here (read-only); leave absent.
            name: None,
            fsrs_parameters: None,
        })
    }
}

/// A titled group of option rows within the settings form.
#[component]
fn options_group(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="deck-options-group">
            <h3 class="deck-options-group__title">{title}</h3>
            <div class="deck-options-group__rows">{children()}</div>
        </div>
    }
}

/// A label + control row. The control is provided as children.
#[component]
fn field_row(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="deck-options-row">
            <span class="deck-options-row__label">{label}</span>
            <div class="deck-options-row__control">{children()}</div>
        </div>
    }
}

/// A numeric text input bound to a string signal (parsed on save).
#[component]
fn number_field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <UiInput value=value input_type="number" placeholder=placeholder />
        </FieldRow>
    }
}

/// A free-text input (Anki-style step strings).
#[component]
fn text_field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <UiInput value=value placeholder=placeholder />
        </FieldRow>
    }
}

/// A boolean toggle row.
#[component]
fn toggle_field(label: &'static str, checked: RwSignal<bool>) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <input type="checkbox" class="toggle" bind:checked=checked />
        </FieldRow>
    }
}

/// An enum selector row, driven by a snake-case string signal.
#[component]
fn select_field(
    label: &'static str,
    value: RwSignal<String>,
    options: &'static [(&'static str, &'static str)],
) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <UiSelect value=value options=options />
        </FieldRow>
    }
}

/// Seven per-weekday Easy Days selectors (Monday-first).
#[component]
fn easy_days_field(days: [RwSignal<String>; 7]) -> impl IntoView {
    const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    view! {
        <div class="deck-options-easy-days">
            {NAMES.iter().enumerate().map(|(i, name)| {
                let d = days[i];
                view! {
                    <div class="deck-options-easy-day">
                        <span class="deck-options-easy-day__label">{*name}</span>
                        <UiSelect value=d options=EASY_DAY_OPTIONS />
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

// --- Enum option lists (wire snake_case value, human label) ---

const NEW_GATHER_OPTIONS: &[(&str, &str)] = &[
    ("deck", "Deck"),
    ("deck_then_random_notes", "Deck, then random notes"),
    ("ascending", "Ascending position"),
    ("descending", "Descending position"),
    ("random_notes", "Random notes"),
    ("random_cards", "Random cards"),
];
const NEW_SORT_OPTIONS: &[(&str, &str)] = &[
    ("card_type_then_gathered", "Card type, then order gathered"),
    ("gathered", "Order gathered"),
    ("card_type_then_random", "Card type, then random"),
    ("random_note_then_card_type", "Random note, then card type"),
    ("random", "Random"),
];
const NEW_REVIEW_OPTIONS: &[(&str, &str)] = &[
    ("mix", "Mix with reviews"),
    ("after", "Show after reviews"),
    ("before", "Show before reviews"),
];
const REVIEW_SORT_OPTIONS: &[(&str, &str)] = &[
    ("due_then_random", "Due date, then random"),
    ("due_then_deck", "Due date, then deck"),
    ("deck_then_due", "Deck, then due date"),
    ("ascending_interval", "Ascending intervals"),
    ("descending_interval", "Descending intervals"),
    ("easy_first", "Easy first"),
    ("difficult_first", "Difficult first"),
    ("ascending_retrievability", "Ascending retrievability"),
    ("descending_retrievability", "Descending retrievability"),
    ("relative_overdueness", "Relative overdueness"),
    ("random", "Random"),
    ("order_added", "Order added"),
    ("latest_added_first", "Latest added first"),
];
const INSERTION_ORDER_OPTIONS: &[(&str, &str)] = &[
    ("sequential", "Sequential"),
    ("random", "Random"),
];
const LEECH_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("tag_only", "Tag only"),
    ("suspend_card", "Suspend card"),
];
const QUESTION_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("show_answer", "Show answer"),
    ("show_card", "Show card"),
];
const ANSWER_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("bury_card", "Bury card"),
    ("answer_again", "Answer again"),
    ("answer_good", "Answer good"),
    ("answer_hard", "Answer hard"),
    ("show_reminder", "Show reminder"),
];
const EASY_DAY_OPTIONS: &[(&str, &str)] = &[
    ("minimum", "Minimum"),
    ("reduced", "Reduced"),
    ("normal", "Normal"),
];

/// Format desired retention (0..1) into a whole-percent string (e.g. 0.9 → "90").
fn format_desired_retention(value: f64) -> String {
    format!("{:.0}", value * 100.0)
}

/// Format a float for an editable input (drop a trailing `.0`, keep 1 dp else).
fn format_float_input(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// Format learning/relearning steps (seconds) back to Anki's compact `1m 10m`
/// notation, so the values read the way the user expects them on the wire.
fn format_steps(steps: &[i64]) -> String {
    if steps.is_empty() {
        return "(none)".to_string();
    }
    steps.iter().map(|&s| format_interval(s)).collect::<Vec<_>>().join(" ")
}

/// Format the raw FSRS weight vector (kept read-only; the optimizer is deferred).
fn format_parameters(params: &[f32]) -> String {
    params
        .iter()
        .map(|w| format!("{w:.4}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A single card: the front (then the revealed back) rendered as HTML in a
/// sandboxed iframe, with the reveal/answer controls pinned to the bottom.
fn study_card_view(
    card: StudyCardView,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let rating = move |r: i32| {
        set_event.set(Event::Answer { rating: r });
    };
    // Predicted intervals (rating -> seconds), if the server returned them.
    let intervals = card.predicted_interval.unwrap_or_default();
    let interval_for = move |r: i32| intervals.get(&r).copied();

    let front = card.front;
    let back = card.back;
    let styling = card.styling;

    view! {
        <div class="study-card-view">
            <iframe
                class="study-card-view__iframe"
                sandbox=""
                srcdoc=move || {
                    if revealed.get() {
                        card_html(&back, &styling)
                    } else {
                        card_html(&front, &styling)
                    }
                }
            ></iframe>

            <div class="study-card-view__controls">
                {move || {
                    if revealed.get() {
                        view! {
                            <div class="thaw-study-answers">
                                {answer_button(rating.clone(), interval_for.clone(), 1, "Again")}
                                {answer_button(rating.clone(), interval_for.clone(), 2, "Hard")}
                                {answer_button(rating.clone(), interval_for.clone(), 3, "Good")}
                                {answer_button(rating.clone(), interval_for.clone(), 4, "Easy")}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <UiButton appearance=UiButtonAppearance::Neutral on_click=move |_| revealed.set(true)>
                                "Show answer"
                            </UiButton>
                        }
                        .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Wrap a card-side HTML fragment (the template-rendered front/back) in a
/// minimal HTML document so it renders standalone in the sandboxed iframe,
/// injecting the note type's shared styling CSS so the card is self-contained.
fn card_html(body: &str, styling: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\" /><style>{styling}</style></head><body>{body}</body></html>"
    )
}

/// A single answer button (label + predicted interval underneath). The "Again"
/// answer (rating 1) renders as a soft danger/error button; Hard/Good/Easy are
/// soft default buttons.
fn answer_button<R, I>(
    rating: R,
    interval_for: I,
    n: i32,
    label: &'static str,
) -> impl IntoView
where
    R: Fn(i32) + Copy + Send + Sync + 'static,
    I: Fn(i32) -> Option<i64> + Clone,
{
    let interval = interval_for(n).map(format_interval).unwrap_or_default();
    // "Again" (rating 1) is a soft danger/error button; Hard/Good/Easy are soft
    // neutral buttons.
    let (appearance, extra_class) = if n == 1 {
        (UiButtonAppearance::Danger, "btn-soft")
    } else {
        (UiButtonAppearance::NeutralSoft, "")
    };

    view! {
        <div class="thaw-study-answer">
            <UiButton appearance=appearance class=extra_class on_click=move |_| rating(n)>
                {label}
            </UiButton>
            <span class="thaw-study-answer__interval">{interval}</span>
        </div>
    }
}

/// Format an interval in seconds using Anki's interval-unit notation:
/// `<1m` → `60s` style, minutes, hours, days, months, years.
fn format_interval(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    if seconds < MINUTE {
        format!("{seconds}s")
    } else if seconds < HOUR {
        format!("{}m", seconds / MINUTE)
    } else if seconds < DAY {
        format!("{}h", seconds / HOUR)
    } else if seconds < MONTH {
        format!("{}d", seconds / DAY)
    } else if seconds < YEAR {
        format!("{}mo", seconds / MONTH)
    } else {
        format!("{}y", seconds / YEAR)
    }
}

/// The "nothing due" completion state (and a manual re-`StartStudy` affordance).
fn study_done_view(set_event: WriteSignal<Event>) -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; gap: 0.75rem; align-items: flex-start;">
            <p style="color: #888;">"Nothing due right now. Nice work!"</p>
            <UiButton appearance=UiButtonAppearance::Primary on_click=move |_| set_event.set(Event::StartStudy)>
                "Check again"
            </UiButton>
        </div>
    }
}

/// Renders a list of sibling decks, recursing into `children`. Each deck row
/// shows an expander (if it has subdecks), an indented title, and three due
/// counts (new = blue, learning = red, review = green) as coloured badges.
fn deck_subtree(
    decks: Vec<DeckSummary>,
    depth: usize,
    open_items: RwSignal<HashSet<i64>>,
    on_navigate: Callback<(i64, DeckView), ()>,
) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_item(deck, depth, open_items, on_navigate))
        .collect_view()
}

/// Renders a single deck row (and, when expanded, its subdecks).
fn deck_item(
    deck: DeckSummary,
    depth: usize,
    open_items: RwSignal<HashSet<i64>>,
    on_navigate: Callback<(i64, DeckView), ()>,
) -> impl IntoView {
    let id = deck.id;
    let has_children = !deck.children.is_empty();
    let studyable = deck.studyable;
    let children = deck.children;
    // Indent the whole title cell (expander + label) by one step per level.
    let indent = format!("padding-left: calc(var(--spacingHorizontalXXL, 24px) * {depth})");

    view! {
        <div
            class="thaw-deck-row"
            class:thaw-deck-row--disabled=move || !studyable
            role="treeitem"
        >
            <div class="thaw-deck-row__title" style=indent>
                {if has_children {
                    let open_items = open_items;
                    view! {
                        <button
                            class="thaw-deck-row__expander"
                            aria-label="toggle subdecks"
                            on:click=move |_| {
                                open_items.update(|set| {
                                    if set.contains(&id) {
                                        set.remove(&id);
                                    } else {
                                        set.insert(id);
                                    }
                                });
                            }
                        >
                            <span
                                class="thaw-deck-row__chevron"
                                style=move || {
                                    if open_items.get().contains(&id) {
                                        "transform: rotate(90deg)".to_string()
                                    } else {
                                        "transform: rotate(0deg)".to_string()
                                    }
                                }
                            >
                                <Icon glyph=LucideGlyph::ChevronRight size="12" />
                            </span>
                        </button>
                    }
                    .into_any()
                } else {
                    // Reserve the same width as the expander so a leaf's label
                    // lines up with a sibling branch's label (hierarchy shows as
                    // a consistent gap between parent and child, not title drift).
                    view! {
                        <span class="thaw-deck-row__expander" aria-hidden="true"></span>
                    }
                    .into_any()
                }}
                {if studyable {
                    let on_navigate = on_navigate;
                    view! {
                        <button
                            class="thaw-deck-row__label thaw-deck-row__label--study"
                            on:click=move |_| on_navigate.run((id, DeckView::Study))
                        >
                            {deck.title}
                        </button>
                    }
                    .into_any()
                } else {
                    // Context-only ancestor (US-2.19): visible in the tree, but
                    // not studyable. Greyed out and non-interactive with a hint.
                    view! {
                        <span
                            class="thaw-deck-row__label thaw-deck-row__label--disabled"
                            title="Shared for context only"
                        >
                            {deck.title}
                        </span>
                    }
                    .into_any()
                }}
            </div>
            <UiBadge color=deck_badge_color(deck.new_count, UiBadgeColor::Brand)>
                {deck.new_count.to_string()}
            </UiBadge>
            <UiBadge color=deck_badge_color(deck.learning_count, UiBadgeColor::Danger)>
                {deck.learning_count.to_string()}
            </UiBadge>
            <UiBadge color=deck_badge_color(deck.review_count, UiBadgeColor::Success)>
                {deck.review_count.to_string()}
            </UiBadge>
            <div class="thaw-deck-row__actions">
                {if studyable {
                    let on_navigate = on_navigate;
                    view! {
                        <button
                            class="thaw-deck-row__gear"
                            aria-label="Deck options"
                            title="Deck options"
                            on:click=move |_| on_navigate.run((id, DeckView::Options))
                        >
                            <Icon glyph=LucideGlyph::Settings size="16" />
                        </button>
                    }
                    .into_any()
                } else {
                    // Context-only ancestor: no options affordance.
                    ().into_any()
                }}
            </div>
        </div>
        {move || {
            if has_children && open_items.get().contains(&id) {
                deck_subtree(children.clone(), depth + 1, open_items, on_navigate).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

/// The badge colour for a due-count cell: a zero count is shown in a neutral
/// grey (`Informative`), otherwise the per-state colour (new = blue, learning =
/// red, review = green).
fn deck_badge_color(count: i64, themed: UiBadgeColor) -> UiBadgeColor {
    if count == 0 {
        UiBadgeColor::Informative
    } else {
        themed
    }
}
