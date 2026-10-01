//! Leptos shell for the Anjuman core.
//!
//! This is the web front-end. It renders the core's `ViewModel` and forwards UI
//! events into `shared::Core<Anjuman>` via a reactive `Effect`. The shell is
//! pure glue: it consumes `ViewModel` and emits `Event`, never holding logic.
//!
//! UI is built from the `thaw` component library. `ConfigProvider` wraps the
//! tree to apply the theme; plain `<div>` is preferred for layout (nesting thaw
//! components that return `impl IntoView` inside `Space`/`Card` trips an
//! `IntoFragment` error).

use leptos::prelude::*;
use shared::app::{
    DeckOptionsEdit, DeckOptionsView, DeckSummary, DeckView, PresetSummary, StudyCardView,
    StudyCountsView,
};
use shared::{Event, ViewModel};
use std::collections::HashSet;
use thaw::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Card, ConfigProvider, Icon, Input, InputType,
    NavDrawer, NavItem, Select, SelectSize, Switch,
};

use crate::core_link;

/// Microsoft Fluent System Icons `Settings` 16-regular, vendored as an
/// `icondata_core::Icon` (there is no published Fluent crate; the official path
/// data comes from microsoft/fluentui-system-icons). Rendered via `thaw::Icon`.
static SETTINGS_16_REGULAR: icondata_core::Icon = &icondata_core::IconData {
    style: None,
    x: None,
    y: None,
    width: None,
    height: None,
    view_box: Some("0 0 16 16"),
    stroke_linecap: None,
    stroke_linejoin: None,
    stroke_width: None,
    stroke: None,
    fill: None, // inherits `currentColor` (thaw's `Icon` default)
    data: r##"<path d="M7.99994 6C6.89537 6 5.99994 6.89543 5.99994 8C5.99994 9.10457 6.89537 10 7.99994 10C9.10451 10 9.99994 9.10457 9.99994 8C9.99994 6.89543 9.10451 6 7.99994 6ZM6.99994 8C6.99994 7.44772 7.44765 7 7.99994 7C8.55222 7 8.99994 7.44772 8.99994 8C8.99994 8.55228 8.55222 9 7.99994 9C7.44765 9 6.99994 8.55228 6.99994 8ZM10.618 4.39833C10.233 4.46825 9.86392 4.21413 9.7937 3.83074L9.53397 2.41496C9.50816 2.27427 9.39961 2.16301 9.25912 2.13325C8.84818 2.04621 8.42685 2.00195 8 2.00195C7.57289 2.00195 7.1513 2.04627 6.74013 2.13341C6.5996 2.16321 6.49104 2.27452 6.46529 2.41527L6.20629 3.8308C6.1994 3.86844 6.18942 3.90551 6.17647 3.9416C6.04476 4.30859 5.6392 4.49978 5.27062 4.36863L3.91115 3.88463C3.77603 3.83652 3.62511 3.87431 3.52891 3.98033C2.96005 4.60729 2.52892 5.34708 2.2672 6.15302C2.22305 6.28899 2.26562 6.43805 2.37502 6.53053L3.47694 7.46206C3.50626 7.48685 3.53352 7.51399 3.55843 7.5432C3.81177 7.84027 3.77528 8.28558 3.47693 8.53783L2.37502 9.46935C2.26562 9.56183 2.22305 9.71089 2.2672 9.84685C2.52892 10.6528 2.96005 11.3926 3.52891 12.0196C3.62511 12.1256 3.77603 12.1634 3.91115 12.1153L5.27068 11.6312C5.30687 11.6184 5.3441 11.6084 5.38196 11.6015C5.76701 11.5316 6.13608 11.7857 6.2063 12.1691L6.46529 13.5846C6.49104 13.7254 6.5996 13.8367 6.74013 13.8665C7.1513 13.9536 7.57289 13.9979 8 13.9979C8.42685 13.9979 8.84818 13.9537 9.25912 13.8666C9.39961 13.8369 9.50816 13.7256 9.53397 13.5849L9.79368 12.1692C9.8006 12.1314 9.81058 12.0944 9.82353 12.0583C9.95524 11.6913 10.3608 11.5001 10.7294 11.6312L12.0888 12.1153C12.224 12.1634 12.3749 12.1256 12.4711 12.0196C13.04 11.3926 13.4711 10.6528 13.7328 9.84685C13.777 9.71089 13.7344 9.56183 13.625 9.46935L12.5231 8.53782C12.4937 8.51303 12.4665 8.48589 12.4416 8.45667C12.1882 8.1596 12.2247 7.71429 12.5231 7.46205L13.625 6.53053C13.7344 6.43805 13.777 6.28899 13.7328 6.15302C13.4711 5.34708 13.04 4.60729 12.4711 3.98033C12.3749 3.87431 12.224 3.83652 12.0888 3.88463L10.7293 4.36865C10.6931 4.38152 10.6559 4.39146 10.618 4.39833ZM3.99863 4.97726L4.93522 5.3107C5.82017 5.62559 6.79872 5.16815 7.11769 4.2794C7.14903 4.19207 7.17324 4.1021 7.18996 4.01078L7.36738 3.04113C7.5757 3.01512 7.78684 3.00195 8 3.00195C8.213 3.00195 8.42397 3.0151 8.63214 3.04107L8.81011 4.01117C8.98053 4.9408 9.87266 5.55003 10.7967 5.38225C10.8877 5.36572 10.9775 5.34176 11.0647 5.31073L12.0014 4.97726C12.2564 5.31084 12.4684 5.67476 12.6319 6.06064L11.8774 6.6984C11.1566 7.30787 11.0675 8.38649 11.6807 9.10555C11.7408 9.17609 11.8067 9.24166 11.8775 9.3015L12.6319 9.93924C12.4684 10.3251 12.2564 10.689 12.0014 11.0226L11.0646 10.6891C10.1797 10.3742 9.20128 10.8317 8.88231 11.7205C8.85096 11.8078 8.82677 11.8978 8.81004 11.9891L8.63214 12.9588C8.42397 12.9848 8.213 12.9979 8 12.9979C7.78684 12.9979 7.5757 12.9847 7.36738 12.9587L7.18994 11.989C7.01965 11.0592 6.12743 10.4498 5.2033 10.6176C5.11227 10.6342 5.0225 10.6581 4.93528 10.6892L3.99863 11.0226C3.74357 10.689 3.53161 10.3251 3.36814 9.93924L4.12257 9.30148C4.84343 8.69201 4.93254 7.61339 4.31933 6.89433C4.25917 6.82378 4.19332 6.75822 4.12254 6.69838L3.36814 6.06064C3.53161 5.67476 3.74357 5.31084 3.99863 4.97726Z"></path>"##,
};

#[component]
pub fn RootComponent() -> impl IntoView {
    let core = core_link::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::RestoreSession);

    // A place to hold the in-progress email/password while the user types
    // (`thaw`'s `Input` drives these signals directly via its `value` prop).
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
    // Developer convenience: sign in with a canned credential pair, no typing.
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
    // Navigation. The core owns the route (`selected_deck` + `selected_deck_view`);
    // these closures just forward the matching events.

    // Back to the list. Shared by the sidebar and breadcrumb.
    let close_deck = move || set_event.set(Event::CloseDeck);

    // Open a deck and land on a sub-view (Study via the row title, Options via
    // the row's gear icon). `Callback` so it can cross the `#[component]`
    // boundary into `DeckList`.
    let navigate = Callback::new(move |(deck_id, view): (i64, DeckView)| match view {
        DeckView::Study => set_event.set(Event::OpenDeck { deck_id }),
        DeckView::Options => set_event.set(Event::OpenDeckOptions { deck_id }),
    });

    view! {
        <ConfigProvider>
            {move || {
                let vm: ViewModel = view.get();
                if vm.authenticated {
                    let selected_deck = vm.selected_deck.clone();
                    view! {
                        <div class="app-shell">
                            <aside class="app-sidebar">
                                <div class="app-sidebar__brand">"Anjuman"</div>
                                <NavDrawer selected_value=RwSignal::new("decks".to_string()) open_categories=RwSignal::new(Vec::<String>::new()) multiple=false>
                                    <NavItem value=RwSignal::new("decks".to_string())>
                                        "Decks"
                                    </NavItem>
                                </NavDrawer>
                                <div class="app-sidebar__footer">
                                    <p class="app-sidebar__user">{vm.display_name.clone()}</p>
                                    <Button appearance=ButtonAppearance::Subtle on_click=logout>
                                        "Log out"
                                    </Button>
                                </div>
                            </aside>

                            <main class="app-content">
                                {if let Some(deck) = selected_deck.clone() {
                                    let deck_title = deck.title.clone();
                                    let counts = vm.counts.clone();
                                    let deck_options_view = vm.deck_options.clone();
                                    let deck_options_error = vm.deck_options_error.clone();
                                    let sub_view = vm.selected_deck_view;
                                    view! {
                                        <div class="app-header">
                                            <Breadcrumb>
                                                <BreadcrumbItem>
                                                    <button class="thaw-breadcrumb-button" on:click=move |_| close_deck()>"Decks"</button>
                                                </BreadcrumbItem>
                                                <BreadcrumbDivider />
                                                <BreadcrumbItem>
                                                    {deck_title}
                                                </BreadcrumbItem>
                                                <BreadcrumbDivider />
                                                <BreadcrumbItem>
                                                    <span class="thaw-breadcrumb-button thaw-breadcrumb-button--current" aria-current="page">
                                                        {match sub_view {
                                                            DeckView::Study => "Study",
                                                            DeckView::Options => "Options",
                                                        }}
                                                    </span>
                                                </BreadcrumbItem>
                                            </Breadcrumb>
                                            {match sub_view {
                                                DeckView::Study => study_count_badges(counts).into_any(),
                                                DeckView::Options => ().into_any(),
                                            }}
                                        </div>
                                        {match sub_view {
                                            DeckView::Study => view! {
                                                <StudyScreen
                                                    vm=vm.clone() revealed=revealed set_event=set_event />
                                            }.into_any(),
                                            DeckView::Options => view! {
                                                <DeckOptionsPanel
                                                    deck_id=deck.id
                                                    options=deck_options_view
                                                    error=deck_options_error
                                                    presets=vm.presets.clone()
                                                    presets_error=vm.presets_error.clone()
                                                    set_event=set_event />
                                            }.into_any(),
                                        }}
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <div class="app-header">
                                            <Breadcrumb>
                                                <BreadcrumbItem>
                                                    <button class="thaw-breadcrumb-button thaw-breadcrumb-button--current" aria-current="page">"Decks"</button>
                                                </BreadcrumbItem>
                                            </Breadcrumb>
                                        </div>
                                        <div class="app-content__pad">
                                            <DeckList vm=vm.clone() on_navigate=navigate />
                                        </div>
                                    }
                                    .into_any()
                                }}
                            </main>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <main class="app-login">
                            <h1>"Anjuman"</h1>
                            <div style="display: flex; gap: 0.5rem; flex-direction: column;">
                                <Input value=email placeholder="Email" />
                                <Input value=password placeholder="Password" />
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
        </ConfigProvider>
    }
}

/// Renders the authenticated user's deck list (title + due counts), with
/// explicit error and empty states. Pure rendering of the `ViewModel` — no
/// logic beyond deciding what the three states look like.
#[component]
fn deck_list(vm: ViewModel, on_navigate: Callback<(i64, DeckView), ()>) -> impl IntoView {
    // Error state — a deck fetch failed.
    if let Some(err) = vm.decks_error {
        return view! {
            <Card>
                <p style="color: #c00;">"Could not load decks: " {err}</p>
            </Card>
        }
        .into_any();
    }

    // Empty state — signed in but no decks yet.
    if vm.decks.is_empty() {
        return view! {
            <Card>
                <p style="color: #888;">"No decks yet. Create one to get started."</p>
            </Card>
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
        <Card>
            <div class="thaw-deck-header" role="row">
                <span class="thaw-deck-col">"Deck"</span>
                <span class="thaw-deck-col">"New"</span>
                <span class="thaw-deck-col">"Learning"</span>
                <span class="thaw-deck-col">"Review"</span>
            </div>
            <div class="thaw-deck-tree" role="tree">
                {deck_subtree(decks, 0, open_items, on_navigate)}
            </div>
        </Card>
    }
    .into_any()
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
    let busy = vm.busy;

    view! {
        <div class="study-screen">
            {study_error.map(|err| view! {
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
            <Badge
                appearance=BadgeAppearance::Tint
                color=BadgeColor::Brand
                size=BadgeSize::Small
            >
                {counts.new_count.to_string()}
            </Badge>
            <Badge
                appearance=BadgeAppearance::Tint
                color=BadgeColor::Danger
                size=BadgeSize::Small
            >
                {(counts.learning_count + counts.relearning_count).to_string()}
            </Badge>
            <Badge
                appearance=BadgeAppearance::Tint
                color=BadgeColor::Success
                size=BadgeSize::Small
            >
                {counts.review_count.to_string()}
            </Badge>
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
    // server report. Editing/saving is disabled for it.
    let read_only = options.id == 0;
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

    // The assign picker: a `Select` bound to the deck's current preset id. On a
    // user change, forward `Event::DeckOptionsAssign`. `last_assigned` guards
    // against firing on the initial render (the core re-fetches after assignment,
    // so `options.id` then matches the newly-assigned id).
    let assign_selection = RwSignal::new(options.id.to_string());
    let last_assigned = StoredValue::new(options.id);
    Effect::new(move |_| {
        let sel = assign_selection.get();
        if let Ok(id) = sel.parse::<i64>() {
            if last_assigned.get_value() != id {
                last_assigned.set_value(id);
                set_event.set(Event::DeckOptionsAssign {
                    deck_id,
                    options_id: id,
                });
            }
        }
    });

    let on_save = move |_| {
        local_error.set(None);
        match state.to_edit() {
            Ok(edit) => set_event.set(Event::DeckOptionsSave(edit)),
            Err(msg) => local_error.set(Some(msg)),
        }
    };

    view! {
        <section class="deck-options" class:deck-options--readonly=read_only>
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
            } else {
                ().into_any()
            }}

            <div class="deck-options-assign">
                <span class="deck-options-assign__label">"Preset"</span>
                <Select value=assign_selection size=SelectSize::Medium>
                    <option value="0">"Default"</option>
                    {presets.iter().map(|p| {
                        let id = p.id.to_string();
                        let name = p.name.clone();
                        view! { <option value=id>{name}</option> }
                    }).collect_view()}
                </Select>
                {presets_error.map(|err| view! {
                    <span class="deck-options__error">{err}</span>
                })}
            </div>

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
                {if read_only {
                    ().into_any()
                } else {
                    view! {
                        <Button appearance=ButtonAppearance::Primary on_click=on_save>
                            "Save"
                        </Button>
                    }
                    .into_any()
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
            <Input value=value placeholder=placeholder input_type=Signal::derive(|| InputType::Number) />
        </FieldRow>
    }
}

/// A free-text input (Anki-style step strings).
#[component]
fn text_field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <Input value=value placeholder=placeholder />
        </FieldRow>
    }
}

/// A boolean toggle row.
#[component]
fn toggle_field(label: &'static str, checked: RwSignal<bool>) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <Switch checked=checked />
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
            <Select value=value size=SelectSize::Medium>
                {options.iter().map(|(val, lbl)| {
                    view! { <option value=*val selected=move || value.get() == *val>{*lbl}</option> }
                }).collect_view()}
            </Select>
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
                        <Select value=d size=SelectSize::Medium>
                            {EASY_DAY_OPTIONS.iter().map(|(val, lbl)| {
                                view! { <option value=*val selected=move || d.get() == *val>{*lbl}</option> }
                            }).collect_view()}
                        </Select>
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
                                {answer_button(
                                    rating.clone(),
                                    interval_for.clone(),
                                    "thaw-study-answers__again",
                                    1,
                                    "Again",
                                )}
                                {answer_button(rating.clone(), interval_for.clone(), "", 2, "Hard")}
                                {answer_button(rating.clone(), interval_for.clone(), "", 3, "Good")}
                                {answer_button(rating.clone(), interval_for.clone(), "", 4, "Easy")}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <Button appearance=ButtonAppearance::Primary on_click=move |_| revealed.set(true)>
                                "Show answer"
                            </Button>
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

/// A single answer button (label + predicted interval underneath).
fn answer_button<R, I>(
    rating: R,
    interval_for: I,
    class: &'static str,
    n: i32,
    label: &'static str,
) -> impl IntoView
where
    R: Fn(i32) + Copy + Send + Sync + 'static,
    I: Fn(i32) -> Option<i64> + Clone,
{
    let interval = interval_for(n).map(format_interval).unwrap_or_default();

    view! {
        <div class="thaw-study-answer">
            <Button class=class appearance=ButtonAppearance::Primary on_click=move |_| rating(n)>
                {label}
            </Button>
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
            <Button appearance=ButtonAppearance::Primary on_click=move |_| set_event.set(Event::StartStudy)>
                "Check again"
            </Button>
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
                            <svg
                                class="thaw-deck-row__chevron"
                                style=move || {
                                    if open_items.get().contains(&id) {
                                        "transform: rotate(90deg)".to_string()
                                    } else {
                                        "transform: rotate(0deg)".to_string()
                                    }
                                }
                                fill="currentColor"
                                aria-hidden="true"
                                width="12"
                                height="12"
                                viewBox="0 0 12 12"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path d="M4.65 2.15a.5.5 0 0 0 0 .7L7.79 6 4.65 9.15a.5.5 0 1 0 .7.7l3.5-3.5a.5.5 0 0 0 0-.7l-3.5-3.5a.5.5 0 0 0-.7 0Z" fill="currentColor"></path>
                            </svg>
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
            <Badge
                appearance=BadgeAppearance::Tint
                color=deck_badge_color(deck.new_count, BadgeColor::Brand)
                size=BadgeSize::Small
            >
                {deck.new_count.to_string()}
            </Badge>
            <Badge
                appearance=BadgeAppearance::Tint
                color=deck_badge_color(deck.learning_count, BadgeColor::Danger)
                size=BadgeSize::Small
            >
                {deck.learning_count.to_string()}
            </Badge>
            <Badge
                appearance=BadgeAppearance::Tint
                color=deck_badge_color(deck.review_count, BadgeColor::Success)
                size=BadgeSize::Small
            >
                {deck.review_count.to_string()}
            </Badge>
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
                            <Icon icon=SETTINGS_16_REGULAR width="1em" height="1em" />
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
fn deck_badge_color(count: i64, themed: BadgeColor) -> BadgeColor {
    if count == 0 {
        BadgeColor::Informative
    } else {
        themed
    }
}
