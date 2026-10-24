//! Select — an owned, unidirectional select (rust-ui `Select` style).
//!
//! Deferred from rust-ui verbatim adoption (docs §10.3): rust-ui's `Select`
//! popover uses an inline `<script>` + global `ScrollLock` JS layer. Ours stays
//! on the standards-based approach — a `FloatingLayer` (CSS anchor + signal-driven
//! click-outside/Esc/scroll-lock) — and is *named* `Select` so the call site reads
//! the same. When/if the JS floating layer is ever adopted, this is replaced
//! whole; call sites don't change.
//!
//! Unlike a native `<select>`, this is a popover control: a trigger button
//! showing the currently-selected label plus a `ChevronDown`, which opens a
//! popover list of selectable items. It is unidirectional — it reads the
//! selected value from a reactive `value` signal and forwards a `String` via
//! `on_change` (read from the view, emit an event, let the core re-render).
//! Options are `(value, label)` pairs.
//!
//! Styled purely with Tailwind utilities + tokens (rust-ui/shadcn convention).

use leptos::prelude::*;
use tw_merge::tw_merge;

use lepticons::{Icon, LucideGlyph};

use super::floating_layer::FloatingLayer;

/// Owned, unidirectional select. `value` is the raw string form of the selected
/// option (e.g. `"1"`, `"normal"`); `options` are `(value, label)` pairs. The
/// trigger shows the selected label + a chevron; clicking opens a popover list
/// (CSS-anchor positioned below the trigger, left-aligned, top-layer so it isn't
/// clipped). Choosing an item forwards `on_change` and closes the popover.
#[component]
pub fn Select(
    value: Signal<String>,
    options: &'static [(&'static str, &'static str)],
    on_change: Option<Callback<String>>,
) -> impl IntoView {
    let selected_label = Signal::derive(move || {
        let current = value.get();
        options
            .iter()
            .find(|(v, _)| *v == current)
            .map(|(_, lbl)| *lbl)
            .unwrap_or("")
    });

    let trigger_class = tw_merge!(
        "flex h-9 w-full items-center justify-between gap-2 whitespace-nowrap rounded-md \
         border border-input bg-transparent px-3 py-1 text-sm shadow-sm \
         focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
    );

    // The trigger slot — the select button (label + chevron).
    let trigger = {
        let trigger_class = trigger_class.clone();
        Box::new(move |_open: Signal<bool>, _toggle: Callback<leptos::ev::MouseEvent>| {
            view! {
                <button type="button" class=trigger_class.clone() aria-haspopup="listbox">
                    <span>{selected_label}</span>
                    <Icon glyph=LucideGlyph::ChevronDown size="16" class="opacity-50" />
                </button>
            }
            .into_any()
        })
    };

    // The content slot — the option list. Choosing forwards `on_change` and closes.
    let content = {
        Box::new(move |_open: Signal<bool>, close: Callback<()>| {
            view! {
                <ul class="m-0 list-none p-0">
                    {options.iter().map(|(v, lbl)| {
                        let choose = {
                            let on_change = on_change.clone();
                            let close = close.clone();
                            move || {
                                if let Some(on_change) = &on_change {
                                    on_change.run((*v).to_string());
                                }
                                close.run(());
                            }
                        };
                        let is_selected = move || value.get() == *v;
                        view! {
                            <li class="m-0">
                                <button
                                    type="button"
                                    class="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground"
                                    class:font-medium=is_selected
                                    on:click=move |_| choose()
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
            }
            .into_any()
        })
    };

    view! {
        <FloatingLayer
            content_class="flex w-full min-w-[--anchor-size-inline] list-none flex-col rounded-md border border-border bg-popover p-1 m-0 text-popover-foreground shadow-md"
            trigger=trigger
            content=content
        />
    }
}
