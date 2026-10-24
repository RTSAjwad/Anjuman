use leptos::prelude::*;

use lepticons::{Icon, LucideGlyph};

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
/// rendered, e.g. the filter sidebar, the table, and the inspector `<dd>`).
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

/// A dropdown flag picker: a square ghost button with a coloured Lucide `Flag`
/// icon as the trigger, and a list of the eight flag values (0 = none, 1–7 the
/// Anki colours). Clicking an item forwards `on_select(value)` and closes the
/// menu. The trigger icon is tinted with the current flag's colour (neutral when
/// unset).
///
/// Open/close is driven by a local `RwSignal`; the menu is `position: absolute`
/// below the trigger within a `relative` wrapper, and closes on selection.
#[component]
pub(crate) fn FlagDropdown(
    id: &'static str,
    flag: Signal<i64>,
    on_select: impl Fn(i64) + Clone + 'static,
) -> impl IntoView {
    // The trigger stroke: the current flag's colour, or a neutral grey when no
    // flag is set (flag 0 maps to "transparent", which would hide the icon).
    let stroke = move || {
        let f = flag.get();
        if f == 0 {
            "currentColor".to_string()
        } else {
            flag_color(f).to_string()
        }
    };

    let open = RwSignal::new(false);
    let toggle = move |_| open.update(|o| *o = !*o);

    let choose = move |f: i64| {
        open.set(false);
        on_select(f);
    };

    // CSS anchor positioning: the trigger names an anchor; the menu is
    // `position: fixed` + `position-anchor` + `position-area` so it renders in
    // the top layer (immune to `overflow` ancestor clipping) and drops below the
    // trigger, left-aligned. `id` must be unique per instance. The menu stays in
    // the DOM and is shown/hidden via a `data-state` attribute + opacity/scale
    // transition (so it animates), note a `display` toggle.
    let anchor = format!("--{id}-anchor");
    let trigger_style = format!("anchor-name:{anchor}");
    let menu_style = format!(
        "position: fixed; position-anchor: {anchor}; position-area: block-end span-inline-start;"
    );

    view! {
        <div>
            <div tabindex="0" role="button" class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent" aria-label="Set flag" style=trigger_style on:click=toggle>
                <Icon glyph=LucideGlyph::Flag size="20" stroke=stroke />
            </div>
            <ul
                class="z-50 flex w-52 flex-col gap-0 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md transition-all duration-200 data-[state=closed]:pointer-events-none data-[state=closed]:scale-95 data-[state=closed]:opacity-0 data-[state=open]:scale-100 data-[state=open]:opacity-100"
                style=menu_style
                data-state=move || if open.get() { "open" } else { "closed" }
            >
                <li>
                    <button class="flex w-full items-center gap-2 rounded-sm px-3 py-1.5 text-sm" class:bg-accent=move || flag.get() == 0 on:click={
                        let choose = choose.clone();
                        move |_| choose(0)
                    }>
                        {flag_swatch(0)}
                    </button>
                </li>
                {[1i64, 2, 3, 4, 5, 6, 7].into_iter().map(|f| {
                    let choose = choose.clone();
                    view! {
                        <li>
                            <button class="flex w-full items-center gap-2 rounded-sm px-3 py-1.5 text-sm" class:bg-accent=move || flag.get() == f on:click=move |_| choose(f)>
                                {flag_swatch(f)}
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}
