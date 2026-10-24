use leptos::prelude::*;

/// Anki flag colour name for a 0-7 value (0 = none).
pub(crate) fn flag_label(flag: i64) -> &'static str {
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
pub(crate) fn flag_color(flag: i64) -> &'static str {
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
/// rendered, e.g. the filter sidebar, and the table's flag column).
pub(crate) fn flag_swatch(flag: i64) -> impl IntoView {
    let label = flag_label(flag);
    let color = flag_color(flag);
    view! {
        <span class="card-browser__flag-sw">
            <span
                class="card-browser__flag-swatch"
                style=format!("background-color: {color}")
            ></span>
            <span>{label}</span>
        </span>
    }
}
