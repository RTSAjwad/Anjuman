//! Button — the primary action primitive, with a typed `variants!` surface
//! (adopted from `rust-ui/leptos-ui`'s `Button`).

use leptos::prelude::*;

// Generate `UiButtonClass` (a `TwClass`), `UiButtonVariant`, and `UiButtonSize`
// (`TwVariant` derives give `Default` + `as_class()`).
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
/// keep their stable API while the underlying classes come from the `variants!`
/// surface.
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
