//! Switch — rust-ui `Switch` (class strings verbatim), adapted for Crux.
//!
//! Adopted from `rust-ui/leptos-ui` `app_crates/registry/src/ui/switch.rs` with
//! its exact Tailwind class strings. **One deliberate divergence** (see
//! `docs/client/STYLING_MIGRATION.md` §10.5): rust-ui's `Switch` owns its own
//! `RwSignal` and toggles it internally on click; we are a Crux shell, so the
//! checked state comes in via `checked: Signal<bool>` and interaction is
//! forwarded via `on_toggle` (a directional event — suspend vs unsuspend, etc.).
//! The core's re-sync drives the visual back to truth (unidirectional).

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned, unidirectional toggle switch (rust-ui `Switch` classes). Drives its
/// checked state from a reactive `checked` signal (`aria-checked` + `data-state`)
/// and forwards interaction via `on_toggle`. The knub is rust-ui's, verbatim.
#[component]
pub fn Switch(
    checked: Signal<bool>,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let state = move || if checked.get() { "checked" } else { "unchecked" };

    // rust-ui track classes, verbatim.
    let track_class = tw_merge!(
        "inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
        class
    );

    view! {
        <button
            data-name="Switch"
            type="button"
            role="switch"
            aria-checked=move || checked.get().to_string()
            aria-label="Toggle"
            data-state=state
            disabled=move || disabled.get()
            class=track_class
            on:click=move |_| on_toggle()
        >
            <span
                data-state=state
                class="block rounded-full ring-0 shadow-lg transition-transform pointer-events-none size-5 bg-background data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
            />
        </button>
    }
}
