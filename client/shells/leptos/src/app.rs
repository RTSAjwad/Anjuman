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
use shared::app::DeckSummary;
use shared::{Event, ViewModel};
use std::collections::HashSet;
use thaw::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Button, ButtonAppearance, Card, ConfigProvider,
    Input, Tree, TreeItem, TreeItemLayout, TreeItemType,
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

    view! {
        <ConfigProvider>
            <main style="max-width: 24rem; margin: 4rem auto; padding: 0 1rem;">
                <h1>"Anjuman"</h1>

                {move || {
                    let vm: ViewModel = view.get();
                    if vm.authenticated {
                        view! {
                            <div style="display: flex; align-items: center; justify-content: space-between; gap: 1rem;">
                                <div>
                                    <p>"Signed in as " <strong>{vm.display_name.clone()}</strong></p>
                                    <p style="color: #888;">{vm.email.clone()}</p>
                                </div>
                                <Button appearance=ButtonAppearance::Subtle on_click=logout>
                                    "Log out"
                                </Button>
                            </div>

                            <DeckList vm=vm.clone() />
                        }.into_any()
                    } else {
                        view! {
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
                        }.into_any()
                    }
                }}
            </main>
        </ConfigProvider>
    }
}

/// Renders the authenticated user's deck list (title + due counts), with
/// explicit error and empty states. Pure rendering of the `ViewModel` — no
/// logic beyond deciding what the three states look like.
#[component]
fn deck_list(vm: ViewModel) -> impl IntoView {
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
    let decks = vm.decks;
    let open_items = RwSignal::new(HashSet::new());
    view! {
        <Card>
            <div class="thaw-deck-header">
                <span class="thaw-deck-col">"Deck"</span>
                <span class="thaw-deck-col">"New"</span>
                <span class="thaw-deck-col">"Learning"</span>
                <span class="thaw-deck-col">"Review"</span>
            </div>
            <Tree open_items=open_items size=thaw::TreeSize::Medium>
                {deck_subtree(decks, 0)}
            </Tree>
        </Card>
    }
    .into_any()
}

/// Renders a list of sibling decks as tree items (recursing into `children`).
/// Each item's layout shows the title on the left and the New/Learning/Review
/// counts as coloured badges on the right (Anki palette: new = blue, learning =
/// red, review/due = green).
fn deck_subtree(decks: Vec<DeckSummary>, depth: usize) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_item(deck, depth))
        .collect_view()
}

/// Renders a single deck (and its subdecks, if any) as a thaw `TreeItem`.
fn deck_item(deck: DeckSummary, depth: usize) -> impl IntoView {
    let item_type = if deck.children.is_empty() {
        TreeItemType::Leaf
    } else {
        TreeItemType::Branch
    };
    let value = deck.id.to_string();
    let children = deck.children;
    // Indent the title only (via inline style) so the count columns stay on a
    // shared right edge; thaw would otherwise pad the whole row and shift the
    // badges. One thaw spacing step (`--spacingHorizontalXXL`, 24px) per level,
    // matching thaw's own indent scale.
    let indent = format!("padding-left: calc(var(--spacingHorizontalXXL, 24px) * {depth})");

    view! {
        <TreeItem item_type=item_type value=value>
            <TreeItemLayout>
                <div class="thaw-deck-row">
                    <span class="thaw-deck-row__title" style=indent>{deck.title}</span>
                    <Badge
                        appearance=BadgeAppearance::Tint
                        color=BadgeColor::Informative
                        size=BadgeSize::Small
                    >
                        {deck.new_count.to_string()}
                    </Badge>
                    <Badge
                        appearance=BadgeAppearance::Tint
                        color=BadgeColor::Danger
                        size=BadgeSize::Small
                    >
                        {deck.learning_count.to_string()}
                    </Badge>
                    <Badge
                        appearance=BadgeAppearance::Tint
                        color=BadgeColor::Success
                        size=BadgeSize::Small
                    >
                        {deck.review_count.to_string()}
                    </Badge>
                </div>
            </TreeItemLayout>
            {if !children.is_empty() {
                // Nested `Tree` (not raw `TreeItem`s) — thaw's `Tree` detects it's
                // inside a `TreeItem` and renders a `Subtree`, which both increments
                // the indent level and wraps the children in a `CollapseTransition`
                // bound to this item's `open` state (so the chevron actually
                // collapses/expands them).
                view! {
                    <Tree>{deck_subtree(children, depth + 1)}</Tree>
                }
                .into_any()
            } else {
                ().into_any()
            }}
        </TreeItem>
    }
}
