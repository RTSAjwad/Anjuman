//! Submit button — an owned form-submit icon button.

use leptos::prelude::*;

/// Owned form-submit button. A square icon button of `type="submit"` (the
/// surrounding form handles submit), used for a search icon inside a `join`.
#[component]
pub fn UiSubmitButton(
    #[prop(optional)] aria_label: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <button type="submit" class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent" aria-label=aria_label>
            {children()}
        </button>
    }
}
