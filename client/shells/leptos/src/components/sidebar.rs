//! SidebarItem — an icon + optional label nav button for the collapsible app
//! rail (§10.4 #6). Anjuman-specific: rust-ui's `Drawer`/`Sheet`/`SideNav` are
//! overlay/route nav, not width-animated icon rails, so the app shell stays ours;
//! this is the repeated nav item extracted for reuse.
//!
//! Controlled: `active` is a reactive signal mirroring the core route, and
//! interaction is forwarded via `on_click` (the core owns navigation).

use leptos::prelude::*;
use lepticons::Icon;

/// A single sidebar nav entry: an icon always, and a label shown only while the
/// rail is expanded.
#[component]
pub fn SidebarItem(
    /// The icon to render.
    icon: lepticons::LucideGlyph,
    /// The text label (hidden while collapsed).
    label: &'static str,
    /// Whether this item is the active route.
    active: Signal<bool>,
    /// Whether the rail is expanded (label visibility).
    expanded: Signal<bool>,
    /// Forwarded on click (the core owns navigation).
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <button
            data-name="SidebarItem"
            class="flex items-center gap-3 rounded-md px-2 py-2 text-sm font-medium"
            class:bg-accent=active
            on:click=on_click
        >
            <Icon glyph=icon size="16" />
            <span class:hidden=move || !expanded.get()>{label}</span>
        </button>
    }
}
