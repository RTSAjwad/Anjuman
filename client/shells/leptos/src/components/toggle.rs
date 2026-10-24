//! Toggle — an owned, unidirectional switch.

use leptos::prelude::*;

/// Owned, unidirectional toggle switch. Drives its `checked` property from a
/// reactive `checked` signal (`prop:checked`) — the idiomatic Leptos way to keep
/// the visual in sync after interaction. A click forwards `on_toggle` (a
/// directional event, e.g. suspend vs unsuspend), and the core's re-sync drives
/// the visual back to truth.
#[component]
pub fn UiToggle(
    checked: Signal<bool>,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <input
            type="checkbox"
            class="peer sr-only"
            prop:checked=move || checked.get()
            disabled=disabled
            on:click=move |_| on_toggle()
        />
    }
}
