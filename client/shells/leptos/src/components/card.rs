//! Card — an owned container surface.
//!
//! Class strings are `rust-ui/leptos-ui`'s `Card`, **verbatim**. We adopt the
//! bare `Card` surface only (no `CardHeader`/`CardTitle`/`CardContent`/… family
//! yet — nothing consumes them; add them verbatim when a caller needs them).
//! See `docs/client/STYLING_MIGRATION.md` §10.
//!
//! The `CardSize::Sm` variant is rust-ui's verbatim matrix but unused until a
//! caller passes it, hence the module-level allow (docs §10.6).
#![allow(dead_code)]

use leptos::prelude::*;
use tw_merge::tw_merge;

/// rust-ui's `CardSize` (verbatim). Kept for a faithful signature; only
/// `Default` is exercised today.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum CardSize {
    #[default]
    Default,
    Sm,
}

/// Owned container card. The standard surface: bordered, rounded, on the card
/// background, flex column with a gap. `size` selects the padding/gap rhythm
/// (rust-ui's `Default`/`Sm`).
#[component]
pub fn Card(
    #[prop(optional, default = CardSize::default())] size: CardSize,
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    let size_classes = match size {
        CardSize::Default => "py-6 gap-4",
        CardSize::Sm => "py-4 gap-3",
    };
    let data_size = match size {
        CardSize::Default => "default",
        CardSize::Sm => "sm",
    };
    let merged = tw_merge!(
        "bg-card text-card-foreground flex flex-col rounded-xl border shadow-sm",
        size_classes,
        class
    );
    view! {
        <div class=merged data-size=data_size data-name="Card">{children()}</div>
    }
}
