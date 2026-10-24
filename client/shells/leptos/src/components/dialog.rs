//! Dialog — an owned shadcn-style modal dialog.
//!
//! Deferred from rust-ui verbatim adoption (docs §10.3): rust-ui's `Dialog` uses
//! a JS portal + `use_lock_body_scroll` (`window.ScrollLock`). Ours stays on the
//! native `<dialog>` approach — `showModal()`/`close()` already give top-layer,
//! `::backdrop` dim, Esc-to-close, and focus trap for free — and we add the one
//! missing behavior (body scroll-lock) via the shared signal-driven helper in
//! `floating_layer`. *Named* `Dialog` so the call site reads the same.

use leptos::prelude::*;

use super::floating_layer::effect_lock_body_scroll;

/// Owned modal dialog. The single source of truth is the `open` signal: it drives
/// `showModal()`/`close()` on a native `<dialog>` reactively. Backdrop-click-to-
/// close is intentionally **not** wired here — doing so would require writing
/// back to the parent's `open` signal (a controlled prop the caller should own);
/// if it's ever wanted, add an `on_close: Callback<()>` prop rather than calling
/// `dialog.close()` imperatively behind the signal's back.
#[component]
pub fn Dialog(open: Signal<bool>, children: Children) -> impl IntoView {
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

    // The native `<dialog>` blocks interaction with the page behind it, but the
    // underlying body can still scroll in some browsers; lock it while open.
    effect_lock_body_scroll(open);

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
