//! Tabs — an owned tab bar + tab item.

use leptos::prelude::*;

/// The tab-bar container. Supply `UiTab` items as children.
#[component]
pub fn UiTabs(children: Children) -> impl IntoView {
    view! {
        <div class="inline-flex items-center gap-1 rounded-lg bg-muted p-1">
            {children()}
        </div>
    }
}

/// A single tab item, with reactive `active` state and a forwarded click.
#[component]
pub fn UiTab(
    active: Signal<bool>,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class="inline-flex items-center justify-center rounded-md px-3 py-1 text-sm font-medium transition-colors"
            class:bg-background=active
            class:text-foreground=active
            class:shadow-sm=active
            class:text-muted-foreground=move || !active.get()
            class:hover:text-foreground=move || !active.get()
            on:click=on_click
        >
            {children()}
        </button>
    }
}
