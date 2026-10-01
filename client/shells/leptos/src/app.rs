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
use shared::app::{DeckOptionsView, DeckSummary, StudyCardView, StudyCountsView};
use shared::{Event, ViewModel};
use std::collections::HashSet;
use thaw::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Card, ConfigProvider, Input, NavDrawer, NavItem,
};

use crate::core_link;

/// The active sub-view of a selected deck, chosen via the breadcrumb. This is
/// shell-local navigation state (the core has no Study/Options split), so it
/// lives here rather than in a `ViewModel` field.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeckView {
    Study,
    Options,
}

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
    let options_for = StoredValue::new(0i64);
    let seen_card = StoredValue::new(0i64);

    // The shell-local sub-view of the selected deck, chosen via the breadcrumb
    // (Decks → [Deck Name] → Study | Options). Opening a deck defaults to
    // `Study`. Transient UI state (the core has no Study/Options split).
    let active_view = RwSignal::new(DeckView::Study);

    Effect::new(move |_| {
        let vm = view.get();

        if let Some(deck) = &vm.selected_deck {
            match active_view.get() {
                DeckView::Study => {
                    // Once per newly-opened deck, request the first due card.
                    if started_for.get_value() != deck.id {
                        started_for.set_value(deck.id);
                        set_event.set(Event::StartStudy);
                    }
                }
                DeckView::Options => {
                    // Once per deck (until re-entered), fetch its preset.
                    if options_for.get_value() != deck.id {
                        options_for.set_value(deck.id);
                        set_event.set(Event::DeckOptionsRequested { deck_id: deck.id });
                    }
                }
            }
        } else {
            // Deck closed: reset the guards so re-opening the *same* deck
            // re-fires the relevant request.
            started_for.set_value(0);
            options_for.set_value(0);
        }

        // Reset the reveal whenever the shown card changes.
        let card_id = vm.current_card.as_ref().map(|c| c.card_id).unwrap_or(0);
        if seen_card.get_value() != card_id {
            seen_card.set_value(card_id);
            revealed.set(false);
        }
    });

    // -----------------------------------------------------------------
    // Navigation state.
    //
    // `active_view` is declared above (before the request-driving Effect). The
    // closures below just write it and dispatch the matching core events.

    // Back to the list (fires `CloseDeck`). Shared by the sidebar and breadcrumb.
    let close_deck = move || {
        active_view.set(DeckView::Study);
        set_event.set(Event::CloseDeck);
    };

    // Open a deck and land on a sub-view (Study via the row title, Options via
    // the row's gear icon). `Callback` so it can cross the `#[component]`
    // boundary into `DeckList`.
    let navigate = Callback::new(move |(deck_id, view): (i64, DeckView)| {
        active_view.set(view);
        set_event.set(Event::OpenDeck { deck_id });
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
                                                        {match active_view.get() {
                                                            DeckView::Study => "Study",
                                                            DeckView::Options => "Options",
                                                        }}
                                                    </span>
                                                </BreadcrumbItem>
                                            </Breadcrumb>
                                            <div class="app-header__actions">
                                                {match active_view.get() {
                                                    DeckView::Study => study_count_badges(counts).into_any(),
                                                    DeckView::Options => ().into_any(),
                                                }}
                                            </div>
                                        </div>
                                        {match active_view.get() {
                                            DeckView::Study => view! {
                                                <StudyScreen
                                                    vm=vm.clone() revealed=revealed set_event=set_event />
                                            }.into_any(),
                                            DeckView::Options => view! {
                                                <DeckOptionsPanel
                                                    options=deck_options_view
                                                    error=deck_options_error />
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

/// The read-only deck-options panel (US-4.9): shows the selected deck's
/// scheduling preset, grouped the way the support matrix groups them, with
/// enums rendered as human labels rather than raw snake_case strings.
///
/// Rendered alongside the study screen; nothing is shown until the preset has
/// been requested (via `Event::DeckOptionsRequested`) and returned — the fetch
/// itself is triggered from the header affordance, so this component only
/// renders whatever the `ViewModel` already carries.
#[component]
fn deck_options_panel(options: Option<DeckOptionsView>, error: Option<String>) -> impl IntoView {
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

    // Force the name into the view so the header renders once (rather than in a
    // closure); the grouped rows below are static given a fixed preset.
    let name = options.name.clone();

    view! {
        <section class="deck-options">
            <header class="deck-options__header">
                <h2 class="deck-options__title">"Deck options"</h2>
                <span class="deck-options__preset">{name}</span>
            </header>

            <OptionsGroup title="Daily limits">
                <OptionRow label="New cards/day" value=options.new_per_day.to_string() />
                <OptionRow label="Maximum reviews/day" value=options.review_per_day.to_string() />
            </OptionsGroup>

            <OptionsGroup title="New cards">
                <OptionRow label="Learning steps" value=format_steps(&options.learning_steps) />
                <OptionRow label="Insertion order" value=label_insertion_order(&options.insertion_order) />
            </OptionsGroup>

            <OptionsGroup title="Lapses">
                <OptionRow label="Relearning steps" value=format_steps(&options.relearning_steps) />
                <OptionRow label="Leech threshold" value=options.leech_threshold.to_string() />
                <OptionRow label="Leech action" value=label_leech_action(&options.leech_action) />
            </OptionsGroup>

            <OptionsGroup title="Display order">
                <OptionRow label="New card gather order" value=label_new_gather_order(&options.new_gather_order) />
                <OptionRow label="New card sort order" value=label_new_sort_order(&options.new_sort_order) />
                <OptionRow label="New/review order" value=label_new_review_order(&options.new_review_order) />
                <OptionRow label="Interday learning/review order" value=label_interday_order(&options.interday_order) />
                <OptionRow label="Review sort order" value=label_review_sort_order(&options.review_sort_order) />
            </OptionsGroup>

            <OptionsGroup title="Burying">
                <OptionRow label="Bury new siblings" value=bool_label(options.bury_new) />
                <OptionRow label="Bury review siblings" value=bool_label(options.bury_review) />
                <OptionRow label="Bury interday learning siblings" value=bool_label(options.bury_interday) />
            </OptionsGroup>

            <OptionsGroup title="Audio">
                <OptionRow label="Don't play audio automatically" value=bool_label(options.dont_play_audio_automatically) />
                <OptionRow label="Skip question when replaying answer" value=bool_label(options.skip_question_when_replaying_answer) />
            </OptionsGroup>

            <OptionsGroup title="Timers">
                <OptionRow label="Maximum answer seconds" value=options.maximum_answer_seconds.to_string() />
                <OptionRow label="Show on-screen timer" value=bool_label(options.show_on_screen_timer) />
                <OptionRow label="Stop on-screen timer on answer" value=bool_label(options.stop_timer_on_answer) />
            </OptionsGroup>

            <OptionsGroup title="Auto advance">
                <OptionRow label="Seconds to show question" value=format_seconds(&options.auto_advance_seconds_show_question) />
                <OptionRow label="Seconds to show answer" value=format_seconds(&options.auto_advance_seconds_show_answer) />
                <OptionRow label="Wait for audio" value=bool_label(options.auto_advance_wait_for_audio) />
                <OptionRow label="Question action" value=label_question_action(&options.auto_advance_question_action) />
                <OptionRow label="Answer action" value=label_answer_action(&options.auto_advance_answer_action) />
            </OptionsGroup>

            <OptionsGroup title="FSRS">
                <OptionRow label="Desired retention" value=format_percent(options.desired_retention) />
                <OptionRow label="Easy days" value=format_easy_days(&options.easy_days) />
            </OptionsGroup>

            <OptionsGroup title="Advanced">
                <OptionRow label="Maximum interval" value=format_days(options.maximum_interval) />
                <OptionRow
                    label="FSRS parameters"
                    value=if options.fsrs_parameters.is_empty() {
                        "Default".to_string()
                    } else {
                        format_parameters(&options.fsrs_parameters)
                    }
                />
            </OptionsGroup>
        </section>
    }
    .into_any()
}

/// A titled group of option rows within the read-only preset.
#[component]
fn options_group(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="deck-options-group">
            <h3 class="deck-options-group__title">{title}</h3>
            <dl class="deck-options-group__rows">{children()}</dl>
        </div>
    }
}

/// A single `label → value` row.
#[component]
fn option_row(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="deck-options-row">
            <dt class="deck-options-row__label">{label}</dt>
            <dd class="deck-options-row__value">{value}</dd>
        </div>
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

/// Format an auto-advance seconds value; `0` disables the feature.
fn format_seconds(value: &f64) -> String {
    if *value <= 0.0 {
        "Off (0 s)".to_string()
    } else if value.fract() == 0.0 {
        format!("{value:.0} s")
    } else {
        format!("{value:.1} s")
    }
}

/// Format desired retention as a whole percent (0.9 → "90%").
fn format_percent(value: f64) -> String {
    format!("{:.0}%", value * 100.0)
}

/// Format the maximum review interval in days, matching Anki's unit.
fn format_days(value: i64) -> String {
    format!("{value} days")
}

/// Format the raw FSRS weight vector (kept read-only; the optimizer is deferred).
fn format_parameters(params: &[f32]) -> String {
    params
        .iter()
        .map(|w| format!("{w:.4}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format Easy Days as a compact Monday-first readout (e.g. "Mon Reduced, …").
fn format_easy_days(days: &[String]) -> String {
    const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    if days.len() != 7 {
        return "(unset)".to_string();
    }
    days.iter()
        .enumerate()
        .map(|(i, d)| format!("{} {}", NAMES[i], label_easy_day(d)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether a boolean option is on/off.
fn bool_label(value: bool) -> String {
    if value {
        "On".to_string()
    } else {
        "Off".to_string()
    }
}

// --- Enum label helpers (snake_case wire name → human label) ---
// These map the serde wire forms exposed on `DeckOptionsView` back to the labels
// Anki uses in-app, so the read-only screen never shows raw debug strings.

fn label_new_gather_order(value: &str) -> String {
    match value {
        "deck" => "Deck".to_string(),
        "deck_then_random_notes" => "Deck, then random notes".to_string(),
        "ascending" => "Ascending position".to_string(),
        "descending" => "Descending position".to_string(),
        "random_notes" => "Random notes".to_string(),
        "random_cards" => "Random cards".to_string(),
        other => humanize(other),
    }
}

fn label_new_sort_order(value: &str) -> String {
    match value {
        "card_type_then_gathered" => "Card type, then order gathered".to_string(),
        "gathered" => "Order gathered".to_string(),
        "card_type_then_random" => "Card type, then random".to_string(),
        "random_note_then_card_type" => "Random note, then card type".to_string(),
        "random" => "Random".to_string(),
        other => humanize(other),
    }
}

fn label_new_review_order(value: &str) -> String {
    match value {
        "mix" => "Mix with reviews".to_string(),
        "after" => "Show after reviews".to_string(),
        "before" => "Show before reviews".to_string(),
        other => humanize(other),
    }
}

fn label_interday_order(value: &str) -> String {
    label_new_review_order(value)
}

fn label_review_sort_order(value: &str) -> String {
    match value {
        "due_then_random" => "Due date, then random".to_string(),
        "due_then_deck" => "Due date, then deck".to_string(),
        "deck_then_due" => "Deck, then due date".to_string(),
        "ascending_interval" => "Ascending intervals".to_string(),
        "descending_interval" => "Descending intervals".to_string(),
        "easy_first" => "Easy first".to_string(),
        "difficult_first" => "Difficult first".to_string(),
        "ascending_retrievability" => "Ascending retrievability".to_string(),
        "descending_retrievability" => "Descending retrievability".to_string(),
        "relative_overdueness" => "Relative overdueness".to_string(),
        "random" => "Random".to_string(),
        "order_added" => "Order added".to_string(),
        "latest_added_first" => "Latest added first".to_string(),
        other => humanize(other),
    }
}

fn label_insertion_order(value: &str) -> String {
    match value {
        "sequential" => "Sequential".to_string(),
        "random" => "Random".to_string(),
        other => humanize(other),
    }
}

fn label_leech_action(value: &str) -> String {
    match value {
        "tag_only" => "Tag only".to_string(),
        "suspend_card" => "Suspend card".to_string(),
        other => humanize(other),
    }
}

fn label_question_action(value: &str) -> String {
    match value {
        "show_answer" => "Show answer".to_string(),
        "show_card" => "Show card".to_string(),
        other => humanize(other),
    }
}

fn label_answer_action(value: &str) -> String {
    match value {
        "bury_card" => "Bury card".to_string(),
        "answer_again" => "Answer again".to_string(),
        "answer_good" => "Answer good".to_string(),
        "answer_hard" => "Answer hard".to_string(),
        "show_reminder" => "Show reminder".to_string(),
        other => humanize(other),
    }
}

fn label_easy_day(value: &str) -> String {
    match value {
        "minimum" => "Minimum".to_string(),
        "reduced" => "Reduced".to_string(),
        "normal" => "Normal".to_string(),
        other => humanize(other),
    }
}

/// Fallback: turn a snake_case wire name into Title Case (used only for values
/// not otherwise mapped — every current enum is mapped above, so this is a
/// defensive path for future enum additions).
fn humanize(snake: &str) -> String {
    snake
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
                            <svg
                                fill="currentColor"
                                aria-hidden="true"
                                width="16"
                                height="16"
                                viewBox="0 0 20 20"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path d="M10 6.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7ZM8 10a2 2 0 1 1 4 0 2 2 0 0 1-4 0Z" fill="currentColor"></path>
                                <path d="M8.13 2.03a1.75 1.75 0 0 1 3.74 0l.13.76c.18.1.35.2.52.32l.72-.28a1.75 1.75 0 0 1 2.1 3.1l-.5.59c.06.2.11.4.15.61l.76.13a1.75 1.75 0 0 1 0 3.48l-.76.13c-.04.21-.09.41-.15.61l.5.59a1.75 1.75 0 0 1-2.1 3.1l-.72-.28c-.17.12-.34.22-.52.32l-.13.76a1.75 1.75 0 0 1-3.74 0l-.13-.76a4.85 4.85 0 0 1-.52-.32l-.72.28a1.75 1.75 0 0 1-2.1-3.1l.5-.59a4.85 4.85 0 0 1-.15-.61l-.76-.13a1.75 1.75 0 0 1 0-3.48l.76-.13c.04-.21.09-.41.15-.61l-.5-.59a1.75 1.75 0 0 1 2.1-3.1l.72.28c.17-.12.34-.22.52-.32l.13-.76Z" fill="none" stroke="currentColor" stroke-width="1.5"></path>
                            </svg>
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
