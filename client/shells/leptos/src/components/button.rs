//! Button — the primary action primitive, with a typed `variants!` surface
//! adopted from `rust-ui/leptos-ui`'s `Button`.
//!
//! The class strings below are rust-ui's, **verbatim** (base + variant + size).
//! `crate::variants!` is our vendored copy of `leptos_ui::variants`.
//!
//! ⚠️ **Divergence to revisit (see docs §10.6):** rust-ui's variant vocabulary is
//! `Default`/`Destructive`/`Outline`/`Secondary`/`Ghost`/`Accent`/`Link`/`Warning`/
//! `Success`/`Bordered`; our callers still speak `Primary`/`Subtle`/`Danger`/
//! `Neutral`/`NeutralSoft`. We keep a thin `ButtonAppearance` enum that renames
//! those onto rust-ui variants (`Primary`→`Default`, `Danger`→`Destructive`,
//! `Neutral`→`Secondary`, `NeutralSoft`→`Ghost`, `Subtle`→`Ghost`). This is a
//! naming-only gap to reconcile later; the *class strings* are rust-ui's already.
//!
//! The generated `ButtonVariant`/`ButtonSize` enums are rust-ui's full verbatim
//! matrix; several variants are intentionally unused until call sites adopt them
//! (see docs §10.6), hence the module-level allow.
#![allow(dead_code)]

use leptos::prelude::*;

crate::variants! {
    Button {
        base: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive w-fit hover:cursor-pointer active:scale-[0.98] active:opacity-100 touch-manipulation [-webkit-tap-highlight-color:transparent] select-none [-webkit-touch-callout:none]",
        variants: {
            variant: {
                Default: "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
                Destructive: "bg-destructive text-white shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60",
                Outline: "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/5",
                Secondary: "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80",
                Ghost: "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
                Accent: "bg-accent text-accent-foreground hover:bg-accent/80",
                Link: "text-primary underline-offset-4 hover:underline",
                Warning: "bg-warning text-warning-foreground hover:bg-warning/90",
                Success: "bg-success text-success-foreground hover:bg-success/90",
                Bordered: "bg-transparent border border-zinc-200 text-muted-foreground",
            },
            size: {
                Default: "h-9 px-4 py-2 has-[>svg]:px-3",
                Sm: "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
                Lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
                Icon: "size-9",
                IconSm: "size-8 rounded-md",
                IconXs: "size-6 rounded-md",
            }
        }
    }
}

/// Our public appearance enumeration (naming divergence — see module doc). Maps
/// onto rust-ui `ButtonVariant` so call sites keep a stable API while the class
/// strings are rust-ui's.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonAppearance {
    Primary,
    Subtle,
    Danger,
    Neutral,
    NeutralSoft,
}

impl From<ButtonAppearance> for ButtonVariant {
    fn from(a: ButtonAppearance) -> Self {
        match a {
            ButtonAppearance::Primary => ButtonVariant::Default,
            ButtonAppearance::Subtle => ButtonVariant::Ghost,
            ButtonAppearance::Danger => ButtonVariant::Destructive,
            ButtonAppearance::Neutral => ButtonVariant::Secondary,
            ButtonAppearance::NeutralSoft => ButtonVariant::Ghost,
        }
    }
}

#[component]
pub fn Button<F>(
    #[prop(default = ButtonAppearance::Primary)] appearance: ButtonAppearance,
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
    let variant: ButtonVariant = appearance.into();
    let size = if square { ButtonSize::Icon } else { ButtonSize::Default };
    let join = if join_item { " rounded-none" } else { "" };
    let class_owned = ButtonClass { variant, size }.with_class(format!("{}{}", class, join));
    view! {
        <button type=button_type class=class_owned disabled=disabled aria-label=aria_label on:click=on_click>
            {children()}
        </button>
    }
}
