//! Flag — the Anki flag colour name/colour/`FlagSwatch` helpers.
//!
//! Anjuman-specific: rust-ui has no Anki-flag swatch (docs §10.4 #5). Shaped to
//! rust-ui conventions (one component per file, `data-name`, `tw_merge!` utility
//! styling, no `<style>` injection). The colour dot is a dynamic value (CSS
//! colour per flag) so it stays an inline `background-color` — the only
//! legitimate inline style here.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Anki flag colour name for a 0-7 value (0 = none).
fn flag_label(flag: i64) -> &'static str {
    match flag {
        0 => "—",
        1 => "Red",
        2 => "Orange",
        3 => "Green",
        4 => "Blue",
        5 => "Pink",
        6 => "Turquoise",
        7 => "Purple",
        _ => "—",
    }
}

/// CSS colour for a 0-7 Anki flag (0 = no colour). Uses CSS named colours.
fn flag_color(flag: i64) -> &'static str {
    match flag {
        1 => "red",
        2 => "orange",
        3 => "green",
        4 => "blue",
        5 => "pink",
        6 => "turquoise",
        7 => "purple",
        _ => "transparent",
    }
}

/// A small coloured dot + label for a flag value (used where free HTML is
/// rendered, e.g. the filter sidebar and the table's flag column).
#[component]
pub fn FlagSwatch(
    flag: i64,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let label = flag_label(flag);
    let color = flag_color(flag);
    let class = tw_merge!("inline-flex items-center gap-2", class);
    view! {
        <span data-name="FlagSwatch" class=class>
            <span
                class="inline-block size-[0.7rem] shrink-0 rounded-full"
                style=format!("background-color: {color}")
            ></span>
            <span>{label}</span>
        </span>
    }
}
