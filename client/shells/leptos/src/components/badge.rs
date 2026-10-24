//! Badge — a small inline label, with a typed `variants!` surface.

use leptos::prelude::*;

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
