//! Select — an owned native `<select>`.

use leptos::prelude::*;

/// Owned select. The canonical spelling of a bordered `<select>`. Call sites
/// supply the `value` signal (two-way `bind:value`) and the `options`.
#[component]
pub fn UiSelect(
    value: RwSignal<String>,
    options: &'static [(&'static str, &'static str)],
) -> impl IntoView {
    view! {
        <select
            class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 \
                   text-sm shadow-sm focus-visible:outline-none focus-visible:ring-[3px] \
                   focus-visible:ring-ring/50"
            bind:value=value
        >
            {options.iter().map(|(val, lbl)| {
                view! { <option value=*val>{*lbl}</option> }
            }).collect_view()}
        </select>
    }
}
