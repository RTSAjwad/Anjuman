//! Breadcrumb — an owned navigation breadcrumb row.
//!
//! Class strings are `rust-ui/leptos-ui`'s `Breadcrumb` family, **verbatim**.
//! We keep the single-wrapper shape (our callers pass raw `<li>` children)
//! rather than rust-ui's full `BreadcrumbList`/`BreadcrumbItem`/`Separator`
//! component family, which nothing consumes yet — add the sub-components
//! verbatim when a caller needs them (rust-ui's `BreadcrumbSeparator` pulls in
//! the `icons` crate). See `docs/client/STYLING_MIGRATION.md` §10.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned breadcrumb row. Renders `<nav>` + `<ol>` navigation with rust-ui's
/// `BreadcrumbList` classes; content is the `<li>` items (call sites add
/// `aria-current` to the trailing crumb).
#[component]
pub fn Breadcrumb(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let list = tw_merge!(
        "flex flex-wrap gap-1 items-center text-sm break-words sm:gap-2 text-muted-foreground",
        class
    );
    view! {
        <nav>
            <ol class=list>{children()}</ol>
        </nav>
    }
}
