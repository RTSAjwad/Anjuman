//! Input — an owned text input.
//!
//! Class strings are `rust-ui/leptos-ui`'s `Input`, **verbatim** (base +
//! focus/aria-invalid/read-only modifiers). rust-ui's `Input` types its value
//! binding via a `strum` `InputType` enum; we keep a plain `&'static str` for
//! `input_type` to avoid pulling in `strum` (docs §10.1). See
//! `docs/client/STYLING_MIGRATION.md` §10.

use leptos::prelude::*;
use tw_merge::tw_merge;

/// Owned text input. The canonical spelling of a bordered form input. Call sites
/// supply `value` (two-way), an optional `input_type`/`placeholder`, and a
/// `class` for layout (`w-full`, `flex-1`, …).
#[component]
pub fn Input(
    value: RwSignal<String>,
    #[prop(optional, default = "text")] input_type: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let merged_class = tw_merge!(
        "text-foreground file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:border-ring focus-visible:ring-ring/50",
        "focus-visible:ring-2",
        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
        "read-only:bg-muted",
        class
    );
    view! {
        <input
            data-name="Input"
            type=input_type
            class=merged_class
            placeholder=placeholder
            bind:value=value
        />
    }
}
