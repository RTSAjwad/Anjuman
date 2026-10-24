//! Tabs — rust-ui `Tabs` family (class strings verbatim), adapted for Crux.
//!
//! Adopted from `rust-ui/leptos-ui` `app_crates/registry/src/ui/tabs.rs` with its
//! exact Tailwind class strings. **One deliberate divergence** (see
//! `docs/client/STYLING_MIGRATION.md` §10.5): rust-ui's `Tabs` owns its selection
//! internally via `TabsCtx { selected: RwSignal<String> }`; we are a Crux shell,
//! so the active tab *must* mirror `ViewModel.route` unidirectionally instead of
//! holding a separate local signal. `TabsTrigger` is therefore **controlled**
//! (`active: Signal<bool>` + `on_click`) rather than `expect_context`-mutating an
//! internal `RwSignal`. `TabsList`/`Tabs` are presentational and drop the
//! context-provision rust-ui needs (no context is required by a controlled
//! trigger).

use leptos::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsVariant {
    #[default]
    Default,
    Line,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsOrientation {
    #[default]
    Horizontal,
}

/// The tab container. verbatim rust-ui.
#[component]
pub fn Tabs(
    #[prop(into, optional)] class: String,
    #[prop(default = TabsOrientation::Horizontal)] orientation: TabsOrientation,
    children: Children,
) -> impl IntoView {
    let is_horizontal = orientation == TabsOrientation::Horizontal;
    let class = tw_merge!(
        "group/tabs flex gap-2",
        if is_horizontal { "flex-col" } else { "flex-row" },
        class
    );

    view! {
        <div
            class=class
            data-name="Tabs"
            data-orientation=if is_horizontal { "Horizontal" } else { "Vertical" }
        >
            {children()}
        </div>
    }
}

/// The tab-bar strip. verbatim rust-ui.
#[component]
pub fn TabsList(
    #[prop(into, optional)] class: String,
    #[prop(default = TabsVariant::Default)] variant: TabsVariant,
    children: Children,
) -> impl IntoView {
    let is_line = variant == TabsVariant::Line;
    let class = tw_merge!(
        "group/tabs-list inline-flex w-fit items-center justify-center rounded-lg p-[3px] text-muted-foreground",
        "group-data-[orientation=Horizontal]/tabs:h-8",
        "group-data-[orientation=Vertical]/tabs:h-fit group-data-[orientation=Vertical]/tabs:flex-col",
        if is_line { "gap-1 bg-transparent rounded-none p-0" } else { "bg-muted" },
        class
    );

    view! {
        <div class=class data-name="TabsList" data-variant=if is_line { "Line" } else { "Default" }>
            {children()}
        </div>
    }
}

/// A single tab trigger. rust-ui's `TabsTrigger`, but **controlled**: the active
/// state comes in via `active: Signal<bool>` and interaction is forwarded via
/// `on_click` (the Crux core owns the selected route). Class strings are
/// rust-ui's, verbatim.
#[component]
pub fn TabsTrigger(
    active: Signal<bool>,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
    children: Children,
) -> impl IntoView {
    // rust-ui base classes (after/underline indicator included), verbatim.
    let base = tw_merge!(
        "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-1.5 py-0.5 text-sm font-medium whitespace-nowrap transition-all cursor-pointer select-none",
        "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none",
        "disabled:pointer-events-none disabled:opacity-50",
        "group-data-[orientation=Vertical]/tabs:w-full group-data-[orientation=Vertical]/tabs:justify-start",
        "after:absolute after:bg-foreground after:transition-opacity",
        "group-data-[orientation=Horizontal]/tabs:after:inset-x-0 group-data-[orientation=Horizontal]/tabs:after:bottom-[-5px] group-data-[orientation=Horizontal]/tabs:after:h-0.5",
        "group-data-[orientation=Vertical]/tabs:after:inset-y-0 group-data-[orientation=Vertical]/tabs:after:-right-1 group-data-[orientation=Vertical]/tabs:after:w-0.5",
    );

    view! {
        <button
            class=move || {
                let is_active = active.get();
                tw_merge!(
                    &base,
                    if is_active { "text-foreground bg-background shadow-sm dark:border-input dark:bg-input/30" }
                    else { "text-foreground/60 hover:text-foreground after:opacity-0" },
                    if is_active { "after:opacity-100" } else { "after:opacity-0" },
                )
            }
            data-name="TabsTrigger"
            data-state=move || if active.get() { "Active" } else { "Inactive" }
            on:click=on_click
        >
            {children()}
        </button>
    }
}
