//! Card — an owned container surface.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned container card. The standard surface: bordered, rounded, on the card
/// background. Content is supplied as children; `class` allows a modifier.
#[component]
pub fn UiCard(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let combined = tw_merge!("rounded-lg border border-border bg-card text-card-foreground shadow-sm", class);
    view! { <div class=combined>{children()}</div> }
}
