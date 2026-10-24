//! Breadcrumbs — an owned navigation breadcrumb row.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned breadcrumb row. Renders `<nav>` + `<ol>` navigation; content is the
/// `<li>` items (call sites add `aria-current` to the trailing crumb).
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
