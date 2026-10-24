//! Leptos shell for the Anjuman core.
//!
//! This is the web front-end. It renders the core's `ViewModel` and forwards UI
//! events into `shared::Core<Anjuman>` via a reactive `Effect`. The shell is
//! pure glue: it consumes `ViewModel` and emits `Event`, never holding logic.
//!
//! UI is styled with Tailwind utilities + the semantic theme tokens in
//! `input.css` (shadcn convention); `src/components/` holds the owned `Ui*`
//! primitives. See `docs/client/STYLING_MIGRATION.md` and `SHELLS.md` §9.

use leptos::prelude::*;
use shared::app::{CurrentRoute, DeckView};
use shared::{Event, ViewModel};

use crate::components::{
    Button, ButtonAppearance, Tabs, TabsList, TabsTrigger, Breadcrumb, Input,
};
use crate::core_link;
use crate::router;
use crate::views::{
    deck_ancestor_titles, study_count_badges, Browser, DeckList, DeckOptionsPanel, StudyControls,
    StudyScreen,
};
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
pub(crate) struct BrowserState {
    // --- Card-view filter sidebar ---
    pub(crate) deck: RwSignal<String>,
    pub(crate) note_type: RwSignal<String>,
    pub(crate) state: RwSignal<String>,
    pub(crate) flag: RwSignal<String>,
    pub(crate) sort: RwSignal<String>,
    pub(crate) query: RwSignal<String>,
    /// Current page (1-based).
    pub(crate) page: RwSignal<usize>,
    /// Guards the one-time "load first page + note types" fetch, so it only
    /// fires when the browser is first shown, not on every ViewModel-driven
    /// re-render (which would reset the user's active filter).
    pub(crate) initialized: RwSignal<bool>,
    // --- Note-view state ---
    /// Note-view filters (deck, note type, sort, search).
    pub(crate) note_deck_filter: RwSignal<String>,
    pub(crate) note_note_type: RwSignal<String>,
    pub(crate) note_sort: RwSignal<String>,
    pub(crate) note_query: RwSignal<String>,
    /// Note-view page (1-based).
    pub(crate) note_page: RwSignal<usize>,
    /// Guards first-fetch of the note list (fires once when Notes view opens).
    pub(crate) notes_initialized: RwSignal<bool>,
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
                        <div class="flex h-svh w-full">
                            // Collapsible sidebar: icon-rail (w-14) <-> expanded (w-64),
                            // driven by the `sidebar_expanded` signal (toggled in the navbar).
                            <aside
                                class="flex flex-col items-stretch border-r border-border bg-muted transition-[width] duration-200"
                                class:w-14=move || !sidebar_expanded.get()
                                class:w-64=sidebar_expanded
                            >
                                <div class="hidden px-3 py-3 font-semibold text-base" class:hidden=move || !sidebar_expanded.get()>"Anjuman"</div>
                                <nav class="flex flex-1 flex-col gap-1 p-2">
                                    <button
                                        class="flex items-center gap-3 rounded-md px-2 py-2 text-sm font-medium"
                                        class:bg-accent=move || route == CurrentRoute::DeckList
                                        on:click=show_deck_list
                                    >
                                        <Icon glyph=LucideGlyph::Library size="16" />
                                        <span class:hidden=move || !sidebar_expanded.get()>"Decks"</span>
                                    </button>
                                    <button
                                        class="flex items-center gap-3 rounded-md px-2 py-2 text-sm font-medium"
                                        class:bg-accent=move || route == CurrentRoute::Cards || route == CurrentRoute::Notes
                                        on:click=show_cards.clone()
                                    >
                                        <Icon glyph=LucideGlyph::Table size="16" />
                                        <span class:hidden=move || !sidebar_expanded.get()>"Browser"</span>
                                    </button>
                                </nav>
                                <div class="mt-auto p-2">
                                    <p class="hidden px-2 pb-2 text-xs text-muted-foreground" class:hidden=move || !sidebar_expanded.get()>{display_name.clone()}</p>
                                    <button
                                        class="flex w-full items-center gap-3 rounded-md px-2 py-2 text-sm font-medium"
                                        on:click=logout
                                    >
                                        <Icon glyph=LucideGlyph::LogOut size="16" />
                                        <span class:hidden=move || !sidebar_expanded.get()>"Log out"</span>
                                    </button>
                                </div>
                            </aside>

                            <div class="flex h-full flex-1 flex-col">
                                <nav class="flex h-12 w-full items-center gap-2 border-b border-border bg-muted px-3">
                                    <button
                                        aria-label="toggle sidebar"
                                        class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent"
                                        on:click=move |_| sidebar_expanded.update(|v| *v = !*v)
                                    >
                                        {move || {
                                            if sidebar_expanded.get() {
                                                view! { <Icon glyph=LucideGlyph::PanelRightClose size="20" /> }.into_any()
                                            } else {
                                                view! { <Icon glyph=LucideGlyph::PanelRightOpen size="20" /> }.into_any()
                                            }
                                        }}
                                    </button>

                                    {match route {
                                        CurrentRoute::DeckList => view! {
                                            <Breadcrumb>
                                                <li><span aria-current="page">"Decks"</span></li>
                                            </Breadcrumb>
                                        }
                                        .into_any(),
                                        CurrentRoute::Cards | CurrentRoute::Notes => view! {
                                            <Tabs>
                                                <TabsList>
                                                    <TabsTrigger
                                                        active=Signal::derive(move || route == CurrentRoute::Cards)
                                                        on_click=show_cards.clone()
                                                    >
                                                        "Cards"
                                                    </TabsTrigger>
                                                    {if can_manage_notes {
                                                        view! {
                                                            <TabsTrigger
                                                                active=Signal::derive(move || route == CurrentRoute::Notes)
                                                                on_click=move |_| set_event.set(Event::ShowNotes)
                                                            >
                                                                "Notes"
                                                            </TabsTrigger>
                                                        }
                                                        .into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </TabsList>
                                            </Tabs>
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
                                                <Breadcrumb class="flex-1">
                                                    <li><button on:click=move |_| close_deck()>"Decks"</button></li>
                                                    {deck_breadcrumb.into_iter().map(|title| view! {
                                                        <li>{title}</li>
                                                    }).collect_view()}
                                                    <li><span aria-current="page">"Study"</span></li>
                                                </Breadcrumb>
                                                {study_count_badges(counts)}
                                                {if let Some(card) = current_card {
                                                    view! {
                                                        <StudyControls card=card set_event=set_event />
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
                                                <Breadcrumb class="flex-1">
                                                    <li><button on:click=move |_| close_deck()>"Decks"</button></li>
                                                    {deck_breadcrumb.into_iter().map(|title| view! {
                                                        <li>{title}</li>
                                                    }).collect_view()}
                                                    <li><span aria-current="page">"Options"</span></li>
                                                </Breadcrumb>
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
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <main class="app-login">
                            <h1>"Anjuman"</h1>
                            <div style="display: flex; gap: 0.5rem; flex-direction: column;">
                                <Input value=email placeholder="Email" />
                                <Input value=password placeholder="Password" input_type="password" />
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=submit
                                    disabled=vm.busy
                                >
                                    "Log in"
                                </Button>
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
                                            <Button
                                                appearance=ButtonAppearance::Subtle
                                                on_click=move |_| q(creds)
                                                disabled=vm.busy
                                            >
                                                {role}
                                            </Button>
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
