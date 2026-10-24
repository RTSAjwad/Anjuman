//! Select — an owned, unidirectional native `<select>`.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned, unidirectional select. The canonical spelling of a bordered
/// `<select>`, matching `UiToggle`/`UiTab`: it drives its selected value from a
/// reactive `value` signal (`prop:value`) and forwards a change via `on_change`
/// (the idiomatic Leptos + Crux unidirectional way — read from the view, emit an
/// event, let the core re-render). `value` holds the raw string form (e.g. `"1"`,
/// `"normal"`); `options` are `(value, label)` pairs.
#[component]
pub fn UiSelect(
    value: Signal<String>,
    options: &'static [(&'static str, &'static str)],
    on_change: Option<Callback<String>>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let combined = tw_merge!(
        "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 \
         text-sm shadow-sm focus-visible:outline-none focus-visible:ring-[3px] \
         focus-visible:ring-ring/50",
        class
    );
    view! {
        <select
            class=combined
            prop:value=move || value.get()
            on:change=move |ev| {
                if let Some(on_change) = &on_change {
                    let v = event_target_value(&ev);
                    on_change.run(v);
                }
            }
        >
            {options.iter().map(|(val, lbl)| {
                view! { <option value=*val>{*lbl}</option> }
            }).collect_view()}
        </select>
    }
}
