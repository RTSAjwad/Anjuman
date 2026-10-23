//! Owned Leptos primitives for the shell's reusable visual units.
//!
//! These are the shell's design-system primitives. They own their **behavior**,
//! **state**, and **structural** markup, and style themselves with **Tailwind
//! utility classes + the semantic theme tokens** in `input.css` (shadcn
//! convention: `bg-primary`, `text-primary-foreground`, `border-input`, …).
//!
//! Styling uses `tw_merge!` so a caller's `class` composes with the primitive's
//! base classes with proper conflict resolution (later string wins).
//!
//! See `docs/client/STYLING_MIGRATION.md` — this is the outcome of removing
//! daisyUI's semantic class bundles in favour of owned, utility-styled
//! components. Nothing here re-implements business logic: the shell is a Crux
//! shell, so it stays unidirectional — read the `ViewModel`, emit an `Event`,
//! let the core re-render. Controls are driven by Leptos signals.

use leptos::prelude::*;
use tw_merge::tw_merge;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

// Generate the typed variant surface for the button: `UiButtonClass` (a
// `TwClass` for `tw_merge` conflict resolution) plus `UiButtonVariant` and
// `UiButtonSize` enums (the `TwVariant` derives give `Default` and `as_class()`).
// Adopted from `rust-ui/leptos-ui`'s `Button` (§9 of STYLING_MIGRATION.md).
crate::variants! {
    UiButton {
        base: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] cursor-pointer active:scale-[0.98] select-none",
        variants: {
            variant: {
                Primary: "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
                Subtle: "hover:bg-accent hover:text-accent-foreground",
                Destructive: "bg-destructive text-white shadow-xs hover:bg-destructive/90",
                Secondary: "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80",
                Muted: "bg-muted text-muted-foreground hover:bg-secondary",
            },
            size: {
                Default: "h-9 px-4 py-2",
                Square: "size-9 px-0 py-0",
            }
        }
    }
}

/// Our public appearance enumeration. Maps to `UiButtonVariant` so call sites
/// keep their stable API while the underlying classes come from the adopted
/// `variants!` surface.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiButtonAppearance {
    Primary,
    Subtle,
    Danger,
    Neutral,
    NeutralSoft,
}

impl From<UiButtonAppearance> for UiButtonVariant {
    fn from(a: UiButtonAppearance) -> Self {
        match a {
            UiButtonAppearance::Primary => UiButtonVariant::Primary,
            UiButtonAppearance::Subtle => UiButtonVariant::Subtle,
            UiButtonAppearance::Danger => UiButtonVariant::Destructive,
            UiButtonAppearance::Neutral => UiButtonVariant::Secondary,
            UiButtonAppearance::NeutralSoft => UiButtonVariant::Muted,
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
    let variant: UiButtonVariant = appearance.into();
    let size = if square { UiButtonSize::Square } else { UiButtonSize::Default };
    let join = if join_item { " rounded-none" } else { "" };
    // `UiButtonClass` resolves conflicts (base + variant + size), then the
    // caller's class and the join modifier are appended last-wins.
    let class_owned = UiButtonClass { variant, size }.with_class(format!("{}{}", class, join));
    view! {
        <button type=button_type class=class_owned disabled=disabled aria-label=aria_label on:click=on_click>
            {children()}
        </button>
    }
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

// Typed badge variant surface (adopted from rust-ui/leptos-ui `Badge`).
crate::variants! {
    UiBadge {
        base: "inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium transition-colors",
        variants: {
            variant: {
                Default: "bg-primary text-primary-foreground",
                Destructive: "bg-destructive text-white",
                Muted: "bg-muted text-muted-foreground",
                Success: "bg-success text-success-foreground",
            }
        }
    }
}

/// Our public badge colour. Maps to `UiBadgeVariant`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiBadgeColor {
    Brand,
    Danger,
    Success,
    Informative,
}

impl From<UiBadgeColor> for UiBadgeVariant {
    fn from(c: UiBadgeColor) -> Self {
        match c {
            UiBadgeColor::Brand => UiBadgeVariant::Default,
            UiBadgeColor::Danger => UiBadgeVariant::Destructive,
            UiBadgeColor::Success => UiBadgeVariant::Success,
            UiBadgeColor::Informative => UiBadgeVariant::Muted,
        }
    }
}

#[component]
pub fn UiBadge(color: UiBadgeColor, children: Children) -> impl IntoView {
    let variant: UiBadgeVariant = color.into();
    let class = UiBadgeClass { variant }.to_class();
    view! { <span class=class>{children()}</span> }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// An owned pager. Renders every page number (the card/note result sets are
/// bounded, so `page_count` stays small). Owns the page-state transitions
/// (`prev`/`next`/`goto`); mutating `page` is observed by the caller (via an
/// `Effect`) and triggers a refetch.
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
        <div class="inline-flex items-center gap-1">
            <button type="button" class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent" aria-label="Previous page" on:click=prev>"‹"</button>
            {(1..=count.get().max(1)).map(|p| {
                let goto = goto.clone();
                let is_active = move || current.get() == p;
                view! {
                    <button
                        type="button"
                        class="inline-flex size-8 items-center justify-center rounded-md text-sm"
                        class:bg-primary=is_active
                        class:text-primary-foreground=is_active
                        class:hover:bg-accent=move || current.get() != p
                        aria-label=move || format!("Page {p}")
                        on:click=move |_| goto(p)
                    >
                        {p.to_string()}
                    </button>
                }
            }).collect_view()}
            <button type="button" class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent" aria-label="Next page" on:click=next>"›"</button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

/// Owned data table for the browser (cards + notes), styled to match
/// rust-ui/leptos-ui's `Table` family. `UiTable` is the `<table>`; compose it
/// with `UiTableHeader`/`UiTableBody`/`UiTableHead`/`UiTableRow`/`UiTableCell`.
/// Call sites still supply the rows/cells (they differ per view); selection is
/// expressed via `UiTableRow`'s `selected` signal.
#[component]
pub fn UiTable(children: Children) -> impl IntoView {
    view! {
        <div class="card-browser__table">
            <table class="w-full text-sm">
                {children()}
            </table>
        </div>
    }
}

/// The `<thead>`, with a bottom border and muted background.
#[component]
pub fn UiTableHeader(children: Children) -> impl IntoView {
    view! {
        <thead class="[&_tr]:border-b bg-muted">
            {children()}
        </thead>
    }
}

/// The `<tbody>` (last row has no bottom border).
#[component]
pub fn UiTableBody(children: Children) -> impl IntoView {
    view! {
        <tbody class="[&_tr:last-child]:border-0">
            {children()}
        </tbody>
    }
}

/// A header cell.
#[component]
pub fn UiTableHead(children: Children) -> impl IntoView {
    view! {
        <th class="h-10 px-3 text-left align-middle font-medium text-muted-foreground">
            {children()}
        </th>
    }
}

/// A data row, with a bottom border, hover state, and optional `selected`
/// highlight + `on_click` (for selectable rows).
#[component]
pub fn UiTableRow(
    children: Children,
    #[prop(optional)] selected: Signal<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <tr
            class="cursor-pointer border-b transition-colors hover:bg-muted/50"
            class:bg-muted=selected
            on:click=move |_| if let Some(cb) = on_click.as_ref() { cb.run(()) }
        >
            {children()}
        </tr>
    }
}

/// A data cell.
#[component]
pub fn UiTableCell(children: Children) -> impl IntoView {
    view! {
        <td class="px-3 py-2 align-middle">
            {children()}
        </td>
    }
}

// ---------------------------------------------------------------------------
// Toggle
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// An owned tab bar. The container owns the tab-bar structure + skin, and each
/// `UiTab` item owns its reactive `active` state. Call sites supply `UiTab`
/// items as children and forward navigation via each tab's `on_click`.
#[component]
pub fn UiTabs(children: Children) -> impl IntoView {
    view! {
        <div class="inline-flex items-center gap-1 rounded-lg bg-muted p-1">
            {children()}
        </div>
    }
}

/// A single tab item: own its active-state styling (from an `active` signal) and
/// forwards a click.
#[component]
pub fn UiTab(
    active: Signal<bool>,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class="inline-flex items-center justify-center rounded-md px-3 py-1 text-sm font-medium transition-colors"
            class:bg-background=active
            class:text-foreground=active
            class:shadow-sm=active
            class:text-muted-foreground=move || !active.get()
            class:hover:text-foreground=move || !active.get()
            on:click=on_click
        >
            {children()}
        </button>
    }
}

// ---------------------------------------------------------------------------
// Input / Select
// ---------------------------------------------------------------------------

/// Owned text input. The canonical spelling of a bordered form input. Call sites
/// supply `value`, an optional `input_type`/`placeholder`, and a `class` for
/// layout (`w-full`, `w-20`, …).
#[component]
pub fn UiInput(
    value: RwSignal<String>,
    #[prop(optional, default = "text")] input_type: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let combined = tw_merge!(
        "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 \
         text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none \
         focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:opacity-50",
        class
    );
    view! {
        <input type=input_type class=combined bind:value=value placeholder=placeholder />
    }
}

/// Owned select. The canonical spelling of a bordered `<select>`. Call sites
/// supply the `value` signal (two-way `bind:value`) and the `options`.
#[component]
pub fn UiSelect(
    value: RwSignal<String>,
    options: &'static [(&'static str, &'static str)],
) -> impl IntoView {
    view! {
        <select
            class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 \
                   text-sm shadow-sm focus-visible:outline-none focus-visible:ring-[3px] \
                   focus-visible:ring-ring/50"
            bind:value=value
        >
            {options.iter().map(|(val, lbl)| {
                view! { <option value=*val>{*lbl}</option> }
            }).collect_view()}
        </select>
    }
}

// ---------------------------------------------------------------------------
// Card / Breadcrumbs / Submit button / Dialog
// ---------------------------------------------------------------------------

/// Owned container card. The standard surface: bordered, rounded, on the card
/// background. Content is supplied as children; `class` allows a modifier.
#[component]
pub fn UiCard(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let combined = tw_merge!("rounded-lg border border-border bg-card text-card-foreground shadow-sm", class);
    view! { <div class=combined>{children()}</div> }
}

/// Owned breadcrumb row. Renders `<ol>`-style navigation; content is the `<li>`
/// items (call sites add `aria-current` to the trailing crumb).
#[component]
pub fn UiBreadcrumbs(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let combined = tw_merge!("flex items-center gap-1 text-sm text-muted-foreground", class);
    view! {
        <nav class=combined>
            <ol class="flex items-center gap-1">
                {children()}
            </ol>
        </nav>
    }
}

/// Owned form-submit button. A square icon button of `type="submit"` (the
/// surrounding form handles submit), used for a search icon inside a `join`.
#[component]
pub fn UiSubmitButton(
    #[prop(optional)] aria_label: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <button type="submit" class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent" aria-label=aria_label>
            {children()}
        </button>
    }
}

/// Owned modal dialog. The single source of truth for a shadcn-style dialog
/// (native `<dialog>` with `showModal()`/`close()` driven reactively from the
/// `open` signal). The `::backdrop` dim and the box entrance are styled here with
/// utilities/tokens, so there is no dependence on a component library's layered
/// open-state.
#[component]
pub fn UiDialog(open: Signal<bool>, children: Children) -> impl IntoView {
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();

    // Drive the native modal state from `open`: `showModal()` on true, `close()`
    // on false. The native API owns the resulting `[open]`/top-layer state.
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
        <dialog
            class="m-auto w-full max-w-lg rounded-lg border-none bg-transparent p-0 backdrop:bg-black/40"
            node_ref=dialog_ref
        >
            <div class="rounded-lg border border-border bg-card p-6 shadow-lg">
                {children()}
            </div>
        </dialog>
    }
}
