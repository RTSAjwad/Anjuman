use leptos::prelude::*;
use shared::app::{DeckSummary, DeckView};
use shared::ViewModel;
use std::collections::HashSet;

use crate::components::{Badge, BadgeColor, Card};
use lepticons::{Icon, LucideGlyph};

/// Renders the authenticated user's deck list (title + due counts), with
/// explicit error and empty states. Pure rendering of the `ViewModel` — no
/// logic beyond deciding what the three states look like.
///
/// Takes `vm` as a reactive `Signal` (not a plain value) so the component
/// re-renders when the core writes new counts into `decks` after study — a plain
/// owned `ViewModel` prop would be snapshotted at first render and never update.
#[component]
pub(crate) fn deck_list(vm: Signal<ViewModel>, on_navigate: Callback<(i64, DeckView), ()>) -> impl IntoView {
    let vm = vm.get();
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
                <span class="thaw-deck-col">"Learn"</span>
                <span class="thaw-deck-col">"Due"</span>
            </div>
            <div class="thaw-deck-tree" role="tree">
                {deck_subtree(decks, 0, open_items, on_navigate)}
            </div>
        </Card>
    }
    .into_any()
}

/// Compute the ancestor chain (root → target, inclusive) of a deck within the
/// nested deck tree, as a list of titles. Used to render full breadcrumb paths
/// for a selected sub-deck (e.g. `Parent > Child > Study`). Returns the empty
/// path when `deck_id` isn't found (the caller falls back to the selected
/// deck's own title, so a missing path is harmless).
pub(crate) fn deck_ancestor_titles(decks: &[DeckSummary], deck_id: i64) -> Vec<String> {
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
            <Badge color=deck_badge_color(deck.new_count, BadgeColor::Brand)>
                {deck.new_count.to_string()}
            </Badge>
            <Badge color=deck_badge_color(deck.learning_count, BadgeColor::Danger)>
                {deck.learning_count.to_string()}
            </Badge>
            <Badge color=deck_badge_color(deck.review_count, BadgeColor::Success)>
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
fn deck_badge_color(count: i64, themed: BadgeColor) -> BadgeColor {
    if count == 0 {
        BadgeColor::Informative
    } else {
        themed
    }
}
