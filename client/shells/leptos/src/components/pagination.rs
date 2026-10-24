//! Pagination — an owned pager.

use leptos::prelude::*;

/// An owned pager. Renders every page number (the card/note result sets are
/// bounded, so `page_count` stays small). Owns the page-state transitions
/// (`prev`/`next`/`goto`); mutating `page` is observed by the caller (via an
/// `Effect`) and triggers a refetch.
#[component]
pub fn UiPagination(page: RwSignal<usize>, page_count: Signal<usize>) -> impl IntoView {
    let count = page_count;
    let current = page;

    let goto = move |p: usize| {
        let max = count.get().max(1);
        current.set(p.clamp(1, max));
    };
    let prev = {
        let goto = goto.clone();
        move |_| goto(current.get().saturating_sub(1).max(1))
    };
    let next = {
        let goto = goto.clone();
        move |_| {
            let max = count.get().max(1);
            goto((current.get() + 1).min(max))
        }
    };

    view! {
        <div class="inline-flex items-center gap-1">
            <button type="button" class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent" aria-label="Previous page" on:click=prev>"‹"</button>
            {(1..=count.get().max(1)).map(|p| {
                let goto = goto.clone();
                let is_active = move || current.get() == p;
                view! {
                    <button
                        type="button"
                        class="inline-flex size-8 items-center justify-center rounded-md text-sm"
                        class:bg-primary=is_active
                        class:text-primary-foreground=is_active
                        class:hover:bg-accent=move || current.get() != p
                        aria-label=move || format!("Page {p}")
                        on:click=move |_| goto(p)
                    >
                        {p.to_string()}
                    </button>
                }
            }).collect_view()}
            <button type="button" class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent" aria-label="Next page" on:click=next>"›"</button>
        </div>
    }
}
