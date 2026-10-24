//! Badge — a small inline label, with a typed `variants!` surface adopted from
//! `rust-ui/leptos-ui`'s `Badge`. Class strings are rust-ui's, verbatim.
//!
//! ⚠️ **Divergence to revisit (see docs §10.6):** rust-ui's `Badge` variants are
//! `Default`/`Secondary`/`Accent`/`Muted`/`Destructive`/`Outline`/`Success`/
//! `Warning`/`Info`; our callers speak `Brand`/`Danger`/`Success`/`Informative`.
//! `BadgeColor` maps `Brand`→`Default`, `Danger`→`Destructive`, `Informative`→
//! `Muted`, `Success`→`Success`. Naming-only gap for later; classes are rust-ui's.
//!
//! The generated `BadgeVariant`/`BadgeSize` enums carry rust-ui's full verbatim
//! matrix; several variants are intentionally unused until call sites adopt them
//! (see docs §10.6), hence the module-level allow.
#![allow(dead_code)]

use leptos::prelude::*;

crate::variants! {
    Badge {
        base: "inline-flex items-center font-semibold rounded-md border transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2 w-fit",
        variants: {
            variant: {
                Default: "border-transparent shadow bg-primary text-primary-foreground hover:bg-primary/80",
                Secondary: "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80",
                Accent: "border-transparent bg-accent text-accent-foreground hover:bg-accent/80",
                Muted: "border-transparent bg-muted text-muted-foreground hover:bg-muted/80",
                Destructive: "border-transparent shadow bg-destructive text-destructive-foreground hover:bg-destructive/80",
                Outline: "text-foreground",
                Success: "border-transparent bg-success-light text-success-dark hover:bg-success-light/80",
                Warning: "border-transparent bg-warning-light text-warning-dark hover:bg-warning-light/80",
                Info: "border-transparent bg-info-light text-info-dark hover:bg-info-light/80",
            },
            size: {
                Default: "px-2.5 py-0.5 text-xs",
                Sm: "px-1.5 py-0.5 text-[10px]",
                Lg: "px-3 py-1 text-sm",
            }
        }
    }
}

/// Our public badge colour (naming divergence — see module doc).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BadgeColor {
    Brand,
    Danger,
    Success,
    Informative,
}

impl From<BadgeColor> for BadgeVariant {
    fn from(c: BadgeColor) -> Self {
        match c {
            BadgeColor::Brand => BadgeVariant::Default,
            BadgeColor::Danger => BadgeVariant::Destructive,
            BadgeColor::Success => BadgeVariant::Success,
            BadgeColor::Informative => BadgeVariant::Muted,
        }
    }
}

#[component]
pub fn Badge(color: BadgeColor, children: Children) -> impl IntoView {
    let variant: BadgeVariant = color.into();
    let class = BadgeClass { variant, size: BadgeSize::Default }.to_class();
    view! { <span class=class>{children()}</span> }
}
