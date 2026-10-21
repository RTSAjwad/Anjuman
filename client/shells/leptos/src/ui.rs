//! Thin Leptos wrappers that map the shell's former `thaw` component usage onto
//! DaisyUI/Tailwind classes.
//!
//! These exist for the few thaw component types that were reused across many call
//! sites (buttons, badges, pagination). Everything else (input, select, toggle,
//! table, card, breadcrumb, nav drawer) is plain HTML with a DaisyUI class and
//! standard Leptos bindings, inlined at the call site — no wrapper needed.
//!
//! Crucially, none of these re-implement thaw's stateful behaviour: the shell is
//! a Crux shell, so it stays unidirectional — read the `ViewModel`, emit an
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

#[component]
pub fn UiButton<F>(
    #[prop(default = UiButtonAppearance::Primary)] appearance: UiButtonAppearance,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: &'static str,
    on_click: F,
    children: Children,
) -> impl IntoView
where
    F: FnMut(leptos::ev::MouseEvent) + 'static,
{
    let appearance_class = match appearance {
        UiButtonAppearance::Primary => "btn btn-primary",
        UiButtonAppearance::Subtle => "btn btn-ghost",
        UiButtonAppearance::Danger => "btn btn-error",
        UiButtonAppearance::Neutral => "btn btn-neutral",
        UiButtonAppearance::NeutralSoft => "btn btn-soft",
    };
    let combined = format!("{appearance_class} {class}");
    view! {
        <button class=combined disabled=disabled on:click=on_click>
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

#[component]
pub fn UiBadge(color: UiBadgeColor, children: Children) -> impl IntoView {
    let color_class = match color {
        UiBadgeColor::Brand => "badge-primary",
        UiBadgeColor::Danger => "badge-error",
        UiBadgeColor::Success => "badge-success",
        UiBadgeColor::Informative => "badge-ghost",
    };
    let combined = format!("badge badge-sm badge-soft {color_class}");
    view! { <span class=combined>{children()}</span> }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// A DaisyUI `join`-styled pager. Unlike thaw's `Pagination`, this renders every
/// page number (the card/note result sets are bounded, so `page_count` stays
/// small in practice). Mutating `page` is observed by the caller (via an
/// `Effect`) and triggers a refetch, so no further callback is needed.
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
            <button class="join-item btn btn-ghost" on:click=prev>"‹"</button>
            {(1..=count.get().max(1)).map(|p| {
                let goto = goto.clone();
                let is_active = move || current.get() == p;
                view! {
                    <button
                        class="join-item btn"
                        class:btn-active=is_active
                        on:click=move |_| goto(p)
                    >
                        {p.to_string()}
                    </button>
                }
            }).collect_view()}
            <button class="join-item btn btn-ghost" on:click=next>"›"</button>
        </div>
    }
}
