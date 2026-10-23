//! Owned Leptos primitives for the shell's reusable visual units.
//!
//! Per `SHELLS.md` §9.5 there are two tracks:
//!
//! - **Structure/state** — owned outright, unlayered and non-`:where()`. These
//!   primitives own their `display`/`position`/flex/state; daisyUI supplies no
//!   structural class.
//! - **Appearance (skin)** — where a daisyUI *visual* bundle (`.btn`, `.badge`)
//!   is the practical source of a look, it is consumed here, centralized behind a
//!   typed enum, **never** spelled as a raw class string at call sites. The
//!   primitive additionally owns the behavior: `<button type=…>`, `disabled`,
//!   ARIA, and events.
//!
//! Nothing here re-implements thaw's or daisyUI's stateful behaviour: the shell
//! is a Crux shell, so it stays unidirectional — read the `ViewModel`, emit an
//! `Event`, let the core re-render. Controls are driven directly by Leptos
//! signals (`bind:` / `prop:` / `on:change`).

use leptos::prelude::*;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiButtonAppearance {
    /// Solid, prominent action.
    Primary,
    /// Transparent until hovered.
    Subtle,
    /// Solid, destructive/danger action (red).
    Danger,
    /// Solid neutral button (daisyUI's explicit `btn-neutral`).
    Neutral,
    /// Soft-tinted default button (`btn btn-soft`).
    NeutralSoft,
}

impl UiButtonAppearance {
    /// The **single** place a daisyUI visual class is named for a button. Call
    /// sites use the enum variant, never this string. (Appearance is the only
    /// thing daisyUI supplies; behaviour — `type`, `disabled`, ARIA, events — is
    /// owned here.)
    fn class(self) -> &'static str {
        match self {
            UiButtonAppearance::Primary => "btn btn-primary",
            UiButtonAppearance::Subtle => "btn btn-ghost",
            UiButtonAppearance::Danger => "btn btn-error",
            UiButtonAppearance::Neutral => "btn btn-neutral",
            UiButtonAppearance::NeutralSoft => "btn btn-soft",
        }
    }
}

#[component]
pub fn UiButton<F>(
    #[prop(default = UiButtonAppearance::Primary)] appearance: UiButtonAppearance,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional)] aria_label: &'static str,
    #[prop(optional)] square: bool,
    #[prop(optional)] join_item: bool,
    #[prop(optional)] class: &'static str,
    on_click: F,
    children: Children,
) -> impl IntoView
where
    F: FnMut(leptos::ev::MouseEvent) + 'static,
{
    let shape = if square { " btn-square" } else { "" };
    let join = if join_item { " join-item" } else { "" };
    let combined = format!("{}{}{} {class}", appearance.class(), shape, join);
    view! {
        <button type=button_type class=combined disabled=disabled aria-label=aria_label on:click=on_click>
            {children()}
        </button>
    }
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiBadgeColor {
    Brand,
    Danger,
    Success,
    Informative,
}

impl UiBadgeColor {
    /// The **single** place a daisyUI visual class is named for a badge. The
    /// base `badge badge-sm badge-soft` shape stays here too, so call sites only
    /// express *which colour*, never the class string.
    fn class(self) -> &'static str {
        let colour = match self {
            UiBadgeColor::Brand => "badge-primary",
            UiBadgeColor::Danger => "badge-error",
            UiBadgeColor::Success => "badge-success",
            UiBadgeColor::Informative => "badge-ghost",
        };
        colour
    }
}

#[component]
pub fn UiBadge(color: UiBadgeColor, children: Children) -> impl IntoView {
    let combined = format!("badge badge-sm badge-soft {}", color.class());
    view! { <span class=combined>{children()}</span> }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// An owned pager. Unlike thaw's `Pagination`, this renders every page number
/// (the card/note result sets are bounded, so `page_count` stays small in
/// practice), and owns the structural layout (a `join` row) + the page-state
/// transitions (`prev`/`next`/`goto`). Mutating `page` is observed by the caller
/// (via an `Effect`) and triggers a refetch, so no further callback is needed.
///
/// The `join`/`btn` classes are the *visual* skin consumed behind this primitive
/// (see §9.5); the paging behaviour and structure are owned here.
#[component]
pub fn UiPagination(page: RwSignal<usize>, page_count: Signal<usize>) -> impl IntoView {
    let count = page_count;
    let current = page;

    let goto = move |p: usize| {
        let max = count.get().max(1);
        current.set(p.clamp(1, max));
    };
    let prev = {
        let goto = goto.clone();
        move |_| goto(current.get().saturating_sub(1).max(1))
    };
    let next = {
        let goto = goto.clone();
        move |_| {
            let max = count.get().max(1);
            goto((current.get() + 1).min(max))
        }
    };

    view! {
        <div class="join">
            <button type="button" class="join-item btn btn-ghost" aria-label="Previous page" on:click=prev>"‹"</button>
            {(1..=count.get().max(1)).map(|p| {
                let goto = goto.clone();
                let is_active = move || current.get() == p;
                view! {
                    <button
                        type="button"
                        class="join-item btn"
                        class:btn-active=is_active
                        aria-label=move || format!("Page {p}")
                        on:click=move |_| goto(p)
                    >
                        {p.to_string()}
                    </button>
                }
            }).collect_view()}
            <button type="button" class="join-item btn btn-ghost" aria-label="Next page" on:click=next>"›"</button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

/// Owned data table for the browser (cards + notes). Owns the structural wrapper
/// (`.card-browser__table` — flex fill + overflow scrolling) and applies the
/// daisyUI `table` *visual* skin centrally. Call sites supply the full
/// `<thead>` + `<tbody>` as children; column/row content and selection remain
/// call-site concerns because they differ per view.
#[component]
pub fn UiTable(children: Children) -> impl IntoView {
    view! {
        <div class="card-browser__table">
            <table class="table">
                {children()}
            </table>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Toggle
// ---------------------------------------------------------------------------

/// Owned, unidirectional toggle switch. Owns the `<input type="checkbox">` + the
/// daisyUI `toggle` *visual* skin, and drives its `checked` **property** from a
/// reactive `checked` signal (`prop:checked`) — the idiomatic Leptos way to keep
/// the visual in sync after interaction, instead of the former `NodeRef`+`Effect`
/// mirroring hack. A click forwards `on_toggle` (a directional event, e.g.
/// suspend vs unsuspend), and the core's re-sync of `checked` drives the visual
/// back to truth. `disabled` is an optional signal.
#[component]
pub fn UiToggle(
    checked: Signal<bool>,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <input
            type="checkbox"
            class="toggle"
            prop:checked=move || checked.get()
            disabled=disabled
            on:click=move |_| on_toggle()
        />
    }
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// An owned tab bar (`tabs tabs-box tabs-sm` in daisyUI): the container owns the
/// tab-bar structure + skin, and each `UiTab` item owns its `tab` class + the
/// reactive `tab-active` state. Call sites supply the `UiTab` items as children
/// and forward navigation via each tab's `on_click`.
#[component]
pub fn UiTabs(children: Children) -> impl IntoView {
    view! {
        <div class="tabs tabs-box tabs-sm">
            {children()}
        </div>
    }
}

/// A single tab item: owns the daisyUI `tab` skin + `tab-active` state (from an
/// `active` signal) and forwards a click.
#[component]
pub fn UiTab(
    active: Signal<bool>,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <button class="tab" class:tab-active=active on:click=on_click>
            {children()}
        </button>
    }
}

// ---------------------------------------------------------------------------
// Input / Select
// ---------------------------------------------------------------------------

/// Owned text input. The **single source of truth** for the daisyUI
/// `input input-bordered` skin: call sites express only `value`, an optional
/// `input_type`/`placeholder`, and a `class` for call-site-specific layout
/// (`w-full`, `join-item`, `w-20`). Two-way binding via `value`.
#[component]
pub fn UiInput(
    value: RwSignal<String>,
    #[prop(optional, default = "text")] input_type: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let combined = format!("input input-bordered {class}");
    view! {
        <input type=input_type class=combined bind:value=value placeholder=placeholder />
    }
}

/// Owned select. The **single source of truth** for the daisyUI
/// `select select-bordered` skin. Call sites supply the `value` signal
/// (two-way `bind:value`) and the `options` (value → label).
#[component]
pub fn UiSelect(
    value: RwSignal<String>,
    options: &'static [(&'static str, &'static str)],
) -> impl IntoView {
    view! {
        <select class="select select-bordered" bind:value=value>
            {options.iter().map(|(val, lbl)| {
                view! { <option value=*val>{*lbl}</option> }
            }).collect_view()}
        </select>
    }
}

// ---------------------------------------------------------------------------
// Card / Breadcrumbs / Submit button
// ---------------------------------------------------------------------------

/// Owned container card. The **single source of truth** for the daisyUI
/// `card bg-base-100 shadow-sm` skin (the app's standard surface). Content is
/// supplied as children; `class` allows a call-site-specific modifier.
#[component]
pub fn UiCard(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let combined = format!("card bg-base-100 shadow-sm {class}");
    view! { <div class=combined>{children()}</div> }
}

/// Owned breadcrumb row. The **single source of truth** for the daisyUI
/// `breadcrumbs` skin + its `<ul>` structure. `class` allows a call-site
/// modifier (e.g. `flex-1`). Content is the `<li>` items.
#[component]
pub fn UiBreadcrumbs(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let combined = format!("breadcrumbs {class}");
    view! {
        <div class=combined>
            <ul>{children()}</ul>
        </div>
    }
}

/// Owned form-submit button. Closes the gap `UiButton` doesn't model: a `<button
/// type="submit">` (no separate click handler — the surrounding form handles the
/// submit) with the daisyUI `btn btn-square join-item` skin, typically used for a
/// search icon inside a `join`.
#[component]
pub fn UiSubmitButton(
    #[prop(optional)] aria_label: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <button type="submit" class="btn btn-square join-item" aria-label=aria_label>
            {children()}
        </button>
    }
}

/// Owned daisyUI modal dialog. The **single source of truth** for the
/// `modal` + `modal-box` structure. Opening/closing uses the native
/// `showModal()`/`close()` API (driven reactively from the `open` signal via a
/// `NodeRef` + `Effect`), which gives daisyUI's dimmed `::backdrop`, top-layer
/// stacking, and the open/close cross-fade — rather than the `open` attribute
/// (which leaks the UA `<dialog>` chrome and doesn't animate).
#[component]
pub fn UiDialog(open: Signal<bool>, children: Children) -> impl IntoView {
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();

    // Drive the native modal state from `open`: `showModal()` on true, `close()`
    // on false. The native API owns the resulting `[open]`/top-layer state, so
    // the signal is the single source of truth and the DOM follows. (Dialogs are
    // inherently imperative in the platform; the imperative call is confined here.)
    Effect::new(move |_| {
        if let Some(dialog) = dialog_ref.get() {
            if open.get() {
                if !dialog.open() {
                    let _ = dialog.show_modal();
                }
            } else {
                dialog.close();
            }
        }
    });

    view! {
        <dialog class="modal" node_ref=dialog_ref>
            <div class="modal-box">
                {children()}
            </div>
        </dialog>
    }
}
