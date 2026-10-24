//! Badge — a small inline label, with a typed `variants!` surface adopted from
//! `rust-ui/leptos-ui`'s `Badge`. Class strings are rust-ui's, verbatim.
//!
//! Two divergences resolved (see docs §10.6):
//! - **Vocabulary** — we use rust-ui's `BadgeVariant` names directly
//!   (`Default`/`Destructive`/`Success`/`Muted`), not the old `Brand`/`Danger`/
//!   `Success`/`Informative` alias.
//! - **Matrix** — trimmed to the variants/sizes Anjuman uses (no `Secondary`/
//!   `Accent`/`Outline`/`Warning`/`Info` variants, no `Sm`/`Lg` sizes), so no dead
//!   code and no `#![allow(dead_code)]`. Add back any entry verbatim when needed.

use leptos::prelude::*;

crate::variants! {
    Badge {
        base: "inline-flex items-center font-semibold rounded-md border transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2 w-fit",
        variants: {
            variant: {
                Default: "border-transparent shadow bg-primary text-primary-foreground hover:bg-primary/80",
                Muted: "border-transparent bg-muted text-muted-foreground hover:bg-muted/80",
                Destructive: "border-transparent shadow bg-destructive text-destructive-foreground hover:bg-destructive/80",
                Success: "border-transparent bg-success-light text-success-dark hover:bg-success-light/80",
            },
            size: {
                Default: "px-2.5 py-0.5 text-xs",
            }
        }
    }
}

#[component]
pub fn Badge(
    #[prop(default = BadgeVariant::Default)] variant: BadgeVariant,
    children: Children,
) -> impl IntoView {
    let class = BadgeClass { variant, size: BadgeSize::Default }.to_class();
    view! { <span class=class>{children()}</span> }
}
