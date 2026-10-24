//! Input — an owned text input.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned text input. The canonical spelling of a bordered form input. Call sites
/// supply `value`, an optional `input_type`/`placeholder`, and a `class` for
/// layout (`w-full`, `w-20`, …).
#[component]
pub fn UiInput(
    value: RwSignal<String>,
    #[prop(optional, default = "text")] input_type: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let combined = tw_merge!(
        "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 \
         text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none \
         focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:opacity-50",
        class
    );
    view! {
        <input type=input_type class=combined bind:value=value placeholder=placeholder />
    }
}
