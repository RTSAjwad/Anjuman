//! Inspector — the shared right-hand details/actions pane shell for the card and
//! note browser (§10.4 #4). Anjuman-specific: rust-ui has no `Card`+`dl`
//! inspector composition. Shaped to rust-ui conventions (one component per file,
//! `data-name`, `tw_merge!` utility styling).
//!
//! Three pieces:
//! - `Inspector` — the `<aside>` surface + title.
//! - `InspectorRow` — a `<dl>` row (`<dt>` label + `<dd>` value), with a `stacked`
//!   variant for the front/back/field text.
//! - `LabeledSwitch` — a `Switch` with an optional inline label (the
//!   suspend/bury toggles), purely reflective (checked is a signal, interaction
//!   emits a directional event).

use leptos::prelude::*;
use tw_merge::tw_merge;

use super::switch::Switch;

/// The inspector surface (right-hand pane). Content is rendered as children.
#[component]
pub fn Inspector(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <aside
            data-name="Inspector"
            class="flex min-h-0 shrink-0 grow-0 basis-80 flex-col gap-3 overflow-y-auto border-l border-border bg-muted p-4"
        >
            <h3 class="m-0 text-base font-semibold">{title}</h3>
            {children()}
        </aside>
    }
}

/// The field list (`<dl>`) the inspector renders.
#[component]
pub fn InspectorFields(children: Children) -> impl IntoView {
    view! {
        <dl data-name="InspectorFields" class="m-0 flex flex-col gap-2">
            {children()}
        </dl>
    }
}

/// A single `<dt>`/`<dd>` row in the inspector. `stacked` stacks the value under
/// the label (used for the card front/back and note field text).
#[component]
pub fn InspectorRow(
    #[prop(into)] label: String,
    #[prop(optional)] stacked: bool,
    children: Children,
) -> impl IntoView {
    if stacked {
        view! {
            <div
                data-name="InspectorRow"
                class="flex flex-col gap-0.5"
            >
                <dt class="shrink-0 font-semibold text-muted-foreground">{label}</dt>
                <dd class="m-0 break-words">{children()}</dd>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div
                data-name="InspectorRow"
                class="flex items-baseline gap-2"
            >
                <dt class="min-w-22 shrink-0 font-semibold text-muted-foreground">{label}</dt>
                <dd class="m-0 break-words">{children()}</dd>
            </div>
        }
        .into_any()
    }
}

/// A reflective suspend/bury toggle: checked state comes only from `checked`
/// (derived from the core), and a click emits a directional event via `on_toggle`.
#[component]
pub fn LabeledSwitch(
    checked: Signal<bool>,
    #[prop(optional)] label: &'static str,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    let class = tw_merge!("inline-flex cursor-pointer items-center gap-2");
    view! {
        <label data-name="LabeledSwitch" class=class>
            <Switch checked=checked on_toggle=on_toggle disabled=disabled />
            {if label.is_empty() {
                ().into_any()
            } else {
                view! { <span class="select-none">{label}</span> }.into_any()
            }}
        </label>
    }
}
