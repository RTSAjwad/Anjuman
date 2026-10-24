//! Flag select — a native `<select>` for the Anki-style 0–7 card flag.

use leptos::prelude::*;

/// Anki flag colour name (0 = none).
fn flag_label(flag: i64) -> &'static str {
    match flag {
        0 => "No flag",
        1 => "Red",
        2 => "Orange",
        3 => "Green",
        4 => "Blue",
        5 => "Pink",
        6 => "Turquoise",
        7 => "Purple",
        _ => "No flag",
    }
}

/// A native `<select>` for the card flag (0 = none, 1–7 the Anki colours). The
/// `value` signal holds the current flag; `on_change` forwards the chosen flag.
#[component]
pub fn UiFlagSelect(value: Signal<i64>, on_change: Callback<i64>) -> impl IntoView {
    view! {
        <select
            class="h-9 rounded-md border border-input bg-background px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
            prop:value=move || value.get().to_string()
            on:change=move |ev| {
                if let Ok(f) = event_target_value(&ev).parse::<i64>() {
                    on_change.run(f);
                }
            }
        >
            {(0..=7).map(|f| {
                view! { <option value=f.to_string()>{flag_label(f)}</option> }
            }).collect_view()}
        </select>
    }
}
