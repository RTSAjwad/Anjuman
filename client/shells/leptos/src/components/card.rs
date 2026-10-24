//! Card — an owned container surface.
//!
//! Class strings are `rust-ui/leptos-ui`'s `Card`, **verbatim**. We adopt the
//! bare `Card` surface only (no `CardHeader`/`CardTitle`/`CardContent`/… family,
//! and no `CardSize` matrix — nothing consumes them; add them verbatim when a
//! caller first needs one). See `docs/client/STYLING_MIGRATION.md` §10.6.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned container card. The standard surface: bordered, rounded, on the card
/// background, flex column with a gap (`size` defaults to rust-ui's `Default`
/// rhythm; the `size` prop is omitted until a caller needs `Sm`).
#[component]
pub fn Card(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    let merged = tw_merge!(
        "bg-card text-card-foreground flex flex-col rounded-xl border shadow-sm",
        "py-6 gap-4", // rust-ui `CardSize::Default`
        class
    );
    view! {
        <div class=merged data-size="default" data-name="Card">{children()}</div>
    }
}
