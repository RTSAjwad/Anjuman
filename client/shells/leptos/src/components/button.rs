//! Button — the action primitive, with a typed `variants!` surface adopted from
//! `rust-ui/leptos-ui`'s `Button`.
//!
//! The class strings below are rust-ui's, **verbatim** (base + variant + size).
//! `crate::variants!` is our vendored copy of `leptos_ui::variants`.
//!
//! Two divergences resolved (see docs §10.6):
//! - **Vocabulary** — we now use rust-ui's `ButtonVariant`/`ButtonSize` names
//!   directly (`Default`/`Ghost`/`Secondary`/`Destructive`), not the old
//!   `Primary`/`Subtle`/`Neutral`/`NeutralSoft`/`Danger` alias.
//! - **Matrix** — the `variants!` matrix is **trimmed to the variants/sizes
//!   Anjuman actually uses** (no `Link`/`Warning`/`Success`/`Bordered` variants,
//!   no `Sm`/`Lg`/`IconSm`/`IconXs` sizes), so there is no dead code and no
//!   `#![allow(dead_code)]`. Add back any entry verbatim when a caller needs it.

use leptos::prelude::*;

crate::variants! {
    Button {
        base: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive w-fit hover:cursor-pointer active:scale-[0.98] active:opacity-100 touch-manipulation [-webkit-tap-highlight-color:transparent] select-none [-webkit-touch-callout:none]",
        variants: {
            variant: {
                Default: "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
                Destructive: "bg-destructive text-white shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60",
                Secondary: "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80",
                Ghost: "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
            },
            size: {
                Default: "h-9 px-4 py-2 has-[>svg]:px-3",
                Icon: "size-9",
            }
        }
    }
}

#[component]
pub fn Button<F>(
    #[prop(default = ButtonVariant::Default)] variant: ButtonVariant,
    #[prop(default = ButtonSize::Default)] size: ButtonSize,
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
    // `square` is a convenience that selects the `Icon` size (a plain icon button);
    // an explicit `size` prop still wins.
    let size = if square { ButtonSize::Icon } else { size };
    let join = if join_item { " rounded-none" } else { "" };
    let class_owned = ButtonClass { variant, size }.with_class(format!("{}{}", class, join));
    view! {
        <button type=button_type class=class_owned disabled=disabled aria-label=aria_label on:click=on_click>
            {children()}
        </button>
    }
}
