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

/// Button size. `Medium` is daisyUI's default (no modifier); `Small` maps to
/// `btn-sm`. Kept as a typed enum so call sites express *which size* rather than
/// spelling `btn-sm`. (A `Large` variant can be added trivially if a future
/// surface needs `btn-lg`.)
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UiButtonSize {
    #[default]
    Medium,
    Small,
}

impl UiButtonSize {
    fn class(self) -> &'static str {
        match self {
            UiButtonSize::Medium => "",
            UiButtonSize::Small => " btn-sm",
        }
    }
}

#[component]
pub fn UiButton<F>(
    #[prop(default = UiButtonAppearance::Primary)] appearance: UiButtonAppearance,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional)] aria_label: &'static str,
    #[prop(optional, default = UiButtonSize::Medium)] size: UiButtonSize,
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
    let combined = format!("{}{}{}{} {class}", appearance.class(), size.class(), shape, join);
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
