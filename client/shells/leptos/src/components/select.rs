//! Select — an owned, unidirectional select (rust-ui `Select` style).
//!
//! Unlike a native `<select>`, this is a popover-based control: a trigger button
//! showing the currently-selected label plus a `ChevronDown`, which opens a
//! CSS-anchor popover of selectable items. It matches `UiToggle`/`UiTab` in
//! being unidirectional — it reads the selected value from a reactive `value`
//! signal and forwards a `String` via `on_change` (read from the view, emit an
//! event, let the core re-render). Options are `(value, label)` pairs.

use leptos::prelude::*;
use tw_merge::tw_merge;

use lepticons::{Icon, LucideGlyph};

use crate::styles::scoped_style;

/// Scoped rules that make the select's interior immune to ancestor leakage
/// (list markers/margins/padding from unlayered author CSS or preflight). These
/// are injected once and scoped under the returned class, so the select renders
/// identically no matter where it is mounted.
const SELECT_CSS: &str = ".anj-ui-select ul{list-style:none;margin:0;padding:0}.anj-ui-select li{margin:0}.anj-ui-select button{margin:0;font:inherit;color:inherit}";

/// Owned, unidirectional select. `value` is the raw string form of the selected
/// option (e.g. `"1"`, `"normal"`); `options` are `(value, label)` pairs. The
/// trigger shows the selected label + a chevron; clicking opens a popover list
/// (CSS-anchor positioned below the trigger, left-aligned, top-layer so it isn't
/// clipped). Choosing an item forwards `on_change` and closes the popover.
#[component]
pub fn UiSelect(
    value: Signal<String>,
    options: &'static [(&'static str, &'static str)],
    on_change: Option<Callback<String>>,
) -> impl IntoView {
    let scope = scoped_style("ui-select", SELECT_CSS);

    let class = tw_merge!(
        "flex h-9 w-full items-center justify-between gap-2 whitespace-nowrap rounded-md \
         border border-input bg-transparent px-3 py-1 text-sm shadow-sm \
         focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
    );

    let selected_label = Signal::derive(move || {
        let current = value.get();
        options
            .iter()
            .find(|(v, _)| *v == current)
            .map(|(_, lbl)| *lbl)
            .unwrap_or("")
    });

    let open = RwSignal::new(false);
    let toggle = move |_| open.update(|o| *o = !*o);

    let choose = move |v: &'static str| {
        open.set(false);
        if let Some(on_change) = &on_change {
            on_change.run(v.to_string());
        }
    };

    // CSS anchor positioning: the trigger names an anchor; the popover is
    // `position: fixed` + `position-anchor` + `position-area` (block-end /
    // inline-start) so it drops below the trigger, left-aligned, top-layer.
    let menu_style = "position: fixed; position-anchor: --ui-select-anchor; position-area: block-end span-inline-start;";

    view! {
        <div class=scope>
            <button
                type="button"
                class=class
                style="anchor-name:--ui-select-anchor"
                aria-haspopup="listbox"
                aria-expanded=move || open.get()
                on:click=toggle
            >
                <span>{selected_label}</span>
                <Icon glyph=LucideGlyph::ChevronDown size="16" class="opacity-50" />
            </button>
            <ul
                class="z-50 flex w-full min-w-[--anchor-size-inline] flex-col rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md transition-all duration-200 data-[state=closed]:pointer-events-none data-[state=closed]:scale-95 data-[state=closed]:opacity-0 data-[state=open]:scale-100 data-[state=open]:opacity-100"
                style=menu_style
                data-state=move || if open.get() { "open" } else { "closed" }
            >
                {options.iter().map(|(v, lbl)| {
                    let choose = choose.clone();
                    let is_selected = move || value.get() == *v;
                    view! {
                        <li>
                            <button
                                type="button"
                                class="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground"
                                class:font-medium=is_selected
                                on:click=move |_| choose(v)
                            >
                                <span>{*lbl}</span>
                                {move || {
                                    if value.get() == *v {
                                        view! { <Icon glyph=LucideGlyph::Check size="14" /> }.into_any()
                                    } else {
                                        ().into_any()
                                    }
                                }}
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}
