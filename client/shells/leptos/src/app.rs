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
use shared::app::{DeckSummary, StudyCardView};
use shared::{Event, ViewModel};
use std::collections::HashSet;
use thaw::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Card, ConfigProvider, Input, NavDrawer, NavItem,
};

use crate::core_link;

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
    // to front-only whenever a different card is shown. `started_for` guards
    // `StartStudy` so it fires exactly once per opened deck (not every re-render).
    let revealed = RwSignal::new(false);
    let started_for = StoredValue::new(0i64);
    let seen_card = StoredValue::new(0i64);
    Effect::new(move |_| {
        let vm = view.get();

        match &vm.selected_deck {
            Some(deck) => {
                // Once per newly-opened deck, request the first due card.
                if started_for.get_value() != deck.id {
                    started_for.set_value(deck.id);
                    set_event.set(Event::StartStudy);
                }
            }
            None => {
                // Deck closed: reset the guard so re-opening the *same* deck
                // re-fires `StartStudy`.
                started_for.set_value(0);
            }
        }

        // Reset the reveal whenever the shown card changes.
        let card_id = vm.current_card.as_ref().map(|c| c.card_id).unwrap_or(0);
        if seen_card.get_value() != card_id {
            seen_card.set_value(card_id);
            revealed.set(false);
        }
    });

    // -----------------------------------------------------------------
    // Sidebar navigation state. "Decks" is the only destination for now; its
    // `NavItem` is always selected, and clicking it (or the breadcrumb's
    // "Decks" link) returns to the list via `Event::CloseDeck`.
    let nav_selected = RwSignal::new("decks".to_string());
    let open_categories = RwSignal::new(Vec::<String>::new());
    // Back to the list (fires `CloseDeck`). Shared by the sidebar and breadcrumb.
    let close_deck = move || set_event.set(Event::CloseDeck);

    view! {
        <ConfigProvider>
            {move || {
                let vm: ViewModel = view.get();
                if vm.authenticated {
                    view! {
                        <div class="app-shell">
                            <aside class="app-sidebar">
                                <div class="app-sidebar__brand">"Anjuman"</div>
                                <NavDrawer selected_value=nav_selected open_categories=open_categories multiple=false>
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
                                {if let Some(deck) = vm.selected_deck.clone() {
                                    let deck_title = deck.title.clone();
                                    view! {
                                        <Breadcrumb>
                                            <BreadcrumbItem>
                                                <button class="thaw-breadcrumb-button" on:click=move |_| close_deck()>"Decks"</button>
                                            </BreadcrumbItem>
                                            <BreadcrumbDivider />
                                            <BreadcrumbItem>
                                                <button class="thaw-breadcrumb-button thaw-breadcrumb-button--current" aria-current="page">{deck_title}</button>
                                            </BreadcrumbItem>
                                        </Breadcrumb>
                                        <StudyScreen
                                            vm=vm.clone() deck=deck revealed=revealed set_event=set_event />
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <Breadcrumb>
                                            <BreadcrumbItem>
                                                <button class="thaw-breadcrumb-button thaw-breadcrumb-button--current" aria-current="page">"Decks"</button>
                                            </BreadcrumbItem>
                                        </Breadcrumb>
                                        <DeckList vm=vm.clone() set_event=set_event />
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
fn deck_list(vm: ViewModel, set_event: WriteSignal<Event>) -> impl IntoView {
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
    // note in SCREENS_SUPPORT.md). Collapsed by default.
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
                {deck_subtree(decks, 0, open_items, set_event)}
            </div>
        </Card>
    }
    .into_any()
}

/// The study screen: one card at a time, reveal-then-answer, plus due counts,
/// a "loading" state (while the first fetch is in flight), a "nothing due"
/// completion state, and the study error surface. Pure rendering of the given
/// `ViewModel` snapshot — study state (`StartStudy` gating, reveal reset)
/// lives in `RootComponent`.
#[component]
fn study_screen(
    vm: ViewModel,
    deck: DeckSummary,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let current = vm.current_card;
    let counts = vm.counts;
    let study_error = vm.study_error;
    let busy = vm.busy;

    view! {
        <Card>
            <div style="display: flex; flex-direction: column; gap: 1rem;">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <h2 style="margin: 0;">{deck.title}</h2>
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
                            {counts.learning_count.to_string()}
                        </Badge>
                        <Badge
                            appearance=BadgeAppearance::Tint
                            color=BadgeColor::Success
                            size=BadgeSize::Small
                        >
                            {counts.review_count.to_string()}
                        </Badge>
                        <Badge
                            appearance=BadgeAppearance::Tint
                            color=BadgeColor::Warning
                            size=BadgeSize::Small
                        >
                            {counts.relearning_count.to_string()}
                        </Badge>
                    </div>
                </div>

                {study_error.map(|err| view! {
                    <p style="color: #c00;">{err}</p>
                })}

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
        </Card>
    }
}

/// A single card: front, then (on demand) the back + answer buttons.
fn study_card_view(
    card: StudyCardView,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let rating = move |r: i32| {
        set_event.set(Event::Answer { rating: r });
    };

    view! {
        <div style="display: flex; flex-direction: column; gap: 0.75rem;">
            <div class="thaw-study-card">
                <span class="thaw-study-card__state">{card.state.clone()}</span>
                <p class="thaw-study-card__front">{card.front.clone()}</p>
            </div>

            {move || {
                if revealed.get() {
                    view! {
                        <div class="thaw-study-card thaw-study-card--answer">
                            <p class="thaw-study-card__back">{card.back.clone()}</p>
                        </div>
                        <div class="thaw-study-answers">
                            <Button class="thaw-study-answers__again" appearance=ButtonAppearance::Primary on_click=move |_| rating(1)>"Again"</Button>
                            <Button appearance=ButtonAppearance::Primary on_click=move |_| rating(2)>"Hard"</Button>
                            <Button appearance=ButtonAppearance::Primary on_click=move |_| rating(3)>"Good"</Button>
                            <Button appearance=ButtonAppearance::Primary on_click=move |_| rating(4)>"Easy"</Button>
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
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_item(deck, depth, open_items, set_event))
        .collect_view()
}

/// Renders a single deck row (and, when expanded, its subdecks).
fn deck_item(
    deck: DeckSummary,
    depth: usize,
    open_items: RwSignal<HashSet<i64>>,
    set_event: WriteSignal<Event>,
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
                    let set_event = set_event;
                    view! {
                        <button
                            class="thaw-deck-row__label thaw-deck-row__label--study"
                            on:click=move |_| set_event.set(Event::OpenDeck { deck_id: id })
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
        </div>
        {move || {
            if has_children && open_items.get().contains(&id) {
                deck_subtree(children.clone(), depth + 1, open_items, set_event).into_any()
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
