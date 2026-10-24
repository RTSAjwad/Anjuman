//! FilterMenu — the browser filter sidebar's collapsible tree (§10.4 #3).
//!
//! Anjuman-specific: the "parent is *also* a selectable filter" deck tree has no
//! rust-ui equivalent (`Collapsible` doesn't make the trigger itself selectable).
//! Shaped to rust-ui conventions (one component per file, `data-name`,
//! `tw_merge!` utility styling), using the native `<details>`/`<summary>` for
//! expansion.
//!
//! The deck tree's non-idiomatic edge — a deck with children can't use its
//! `<summary>` as a filter (it only toggles), so the parent's own selectable
//! entry is the first item *inside* the submenu, labelled "this deck only" —
//! is the recorded divergence from §10.4.

use leptos::prelude::*;
use shared::app::DeckSummary;

/// A collapsible filter category (`<details>`/`<summary>` wrapping a `<ul>` of
/// leaves). A plain `fn` (not `#[component]`) so it sits inline in the reactive
/// `view!` and re-renders with async-loaded data.
pub fn filter_branch(label: &'static str, leaves: impl IntoView) -> impl IntoView {
    view! {
        <li data-name="FilterBranch">
            <details>
                <summary class="grid list-none">{label}</summary>
                <ul class="ml-4 list-none pl-2">{leaves}</ul>
            </details>
        </li>
    }
}

/// A single selectable filter leaf (native `<button>`), highlighted with the
/// accent background when selected.
pub fn filter_leaf(
    active: bool,
    label: String,
    on_select: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <li data-name="FilterLeaf">
            <button
                class="flex w-full rounded-sm px-3 py-1.5 text-sm"
                class:bg-accent=active
                on:click=move |_| on_select()
            >
                {label}
            </button>
        </li>
    }
}

/// Renders the deck hierarchy as nested collapsible groups. A deck with children
/// is an expandable `<details>`/`<summary>`; because the summary toggles the
/// submenu (it can't also be a filter), the parent deck's own selectable entry is
/// the first item inside its submenu (labelled "this deck only"), followed by its
/// children. A leaf deck is a plain selectable leaf.
pub fn deck_filter_tree(
    decks: Vec<DeckSummary>,
    active: RwSignal<String>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_filter_node(deck, active.clone(), on_select.clone()))
        .collect_view()
}

/// A single deck node in the filter tree.
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
        // the expanded submenu.
        let self_item = view! {
            <li>
                <button
                    class="flex w-full items-center gap-2 rounded-sm px-3 py-1.5 text-sm"
                    class:bg-accent=is_active.clone()
                    on:click=move |_| select_self(self_id.clone())
                >
                    {title.clone()}
                    <span class="text-xs font-normal text-muted-foreground">{"this deck only"}</span>
                </button>
            </li>
        };
        view! {
            <li data-name="DeckFilterNode">
                <details>
                    <summary class="grid list-none">{title}</summary>
                    <ul class="ml-4 list-none pl-2">
                        {self_item}
                        {deck_filter_tree(children, active.clone(), on_select)}
                    </ul>
                </details>
            </li>
        }
        .into_any()
    } else {
        view! {
            <li data-name="DeckFilterNode">
                <button
                    class="flex w-full rounded-sm px-3 py-1.5 text-sm"
                    class:bg-accent=is_active
                    on:click=move |_| on_select(id_str.clone())
                >
                    {title}
                </button>
            </li>
        }
        .into_any()
    }
}
