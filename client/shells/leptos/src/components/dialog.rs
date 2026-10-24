//! Dialog — an owned shadcn-style modal dialog.

use leptos::prelude::*;

/// Owned modal dialog. The single source of truth for a shadcn-style dialog
/// (native `<dialog>` with `showModal()`/`close()` driven reactively from the
/// `open` signal). The `::backdrop` dim and the box are styled with utilities,
/// so there is no dependence on a component library's layered open-state.
#[component]
pub fn UiDialog(open: Signal<bool>, children: Children) -> impl IntoView {
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();

    // Drive the native modal state from `open`: `showModal()` on true, `close()`
    // on false. The native API owns the resulting `[open]`/top-layer state.
    Effect::new(move |_| {
        if let Some(dialog) = dialog_ref.get() {
            if open.get() {
                if !dialog.open() {
                    let _ = dialog.show_modal();
                }
            } else {
                dialog.close();
            }
        }
    });

    view! {
        <dialog
            class="m-auto w-full max-w-lg rounded-lg border-none bg-transparent p-0 backdrop:bg-black/40"
            node_ref=dialog_ref
        >
            <div class="rounded-lg border border-border bg-card p-6 shadow-lg">
                {children()}
            </div>
        </dialog>
    }
}
