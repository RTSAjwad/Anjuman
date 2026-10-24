//! FloatingLayer — the shared open/close lifecycle for popovers and menus.
//!
//! rust-ui's `Select`/`Dialog`/`DropdownMenu` ship inline `<script>` fragments +
//! a global `window.ScrollLock` to handle click-outside, Esc, and body
//! scroll-lock. We deliberately reject that layer (it mutates `data-state`/
//! `pointer-events` *behind* Leptos's back, reintroducing two-source-of-truth
//! state) — see `docs/client/STYLING_MIGRATION.md` §10.3.
//!
//! This component instead owns the *same* behaviors via Leptos signals and the
//! `window_event_listener` helper:
//!   - **click-outside** to dismiss (a `mousedown` on the window whose target is
//!     not inside the trigger or the popover);
//!   - **Esc** to dismiss;
//!   - **body scroll-lock** while open (`document.body` gets `overflow: hidden`);
//!   - a **unique CSS anchor name** per instance (so two popovers can't resolve
//!     each other's anchor).
//!
//! It is **controlled**: it owns the `open` signal internally and exposes it in
//! both render slots — `trigger(open, toggle)` and `content(open, close)` — so a
//! call site can drive *its own* markup from the open state without holding a
//! second, forkable source of truth. Every transition fires `on_open_change` for
//! callers that must reflect state elsewhere.
//!
//! Styled purely with Tailwind utilities + theme tokens (shadcn convention).

use std::sync::atomic::{AtomicU64, Ordering};

use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

/// A process-wide counter so each floating layer gets a unique CSS-anchor name
/// (`anchor-name` must be unique per page).
static NEXT_ANCHOR: AtomicU64 = AtomicU64::new(0);

/// A unique CSS anchor name in `--…` custom-property form.
fn next_anchor_name() -> String {
    format!("--floating-anchor-{}", NEXT_ANCHOR.fetch_add(1, Ordering::Relaxed))
}

/// Lock the body scroll while `open` is true (adds `/` removes `overflow-hidden`
/// on `document.body`). This is the signal-driven replacement for rust-ui's
/// global `window.ScrollLock` (no inline `<script>`, no JS object). Wire it once
/// in a component by calling `lock_body_scroll(open_signal)` inside an `Effect`.
pub(crate) fn effect_lock_body_scroll(open: Signal<bool>) {
    Effect::new(move |_| {
        let Some(body) = document().body() else { return };
        if open.get() {
            let _ = body.class_list().add_1("overflow-hidden");
            on_cleanup(move || {
                let _ = body.class_list().remove_1("overflow-hidden");
            });
        }
    });
}

/// The shared floating-layer primitive.
///
/// `trigger(open, toggle)` and `content(open, close)` are render slots; the
/// trigger is rendered with the anchor name (clicking it toggles), and the
/// content is a positioned popover. `open` is the reactive open state and
/// `close` dismisses (used by e.g. a menu item to close after selection).
#[component]
pub fn FloatingLayer(
    /// The trigger (clicked to toggle). Receives the reactive `open` state and a
    /// `toggle` callback.
    trigger: Box<dyn Fn(Signal<bool>, Callback<leptos::ev::MouseEvent>) -> AnyView>,
    /// The popover content. Receives the reactive `open` state and a `close`
    /// callback (for dismiss-on-select).
    content: Box<dyn Fn(Signal<bool>, Callback<()>) -> AnyView>,
    /// Extra Tailwind classes for the popover, merged over the base.
    #[prop(into, optional)] content_class: String,
    /// Called with the new `open` state on every transition (optional).
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
) -> impl IntoView {
    let anchor = next_anchor_name();
    let open = RwSignal::new(false);

    let trigger_ref = NodeRef::<leptos::html::Div>::new();
    let menu_ref = NodeRef::<leptos::html::Div>::new();

    let toggle = {
        let open = open;
        let on_open_change = on_open_change.clone();
        Callback::new(move |_: leptos::ev::MouseEvent| {
            let next = !open.get();
            open.set(next);
            if let Some(cb) = &on_open_change {
                cb.run(next);
            }
        })
    };
    let close = {
        let open = open;
        let on_open_change = on_open_change;
        Callback::new(move |()| {
            if open.get() {
                open.set(false);
                if let Some(cb) = &on_open_change {
                    cb.run(false);
                }
            }
        })
    };

    let trigger_style = format!("anchor-name:{anchor}");
    let menu_style =
        format!("position: fixed; position-anchor: {anchor}; position-area: block-end span-inline-start;");

    let open_signal: Signal<bool> = open.into();

    // Click-outside + Esc, only while open. `window_event_listener` returns a
    // `WindowListenerHandle`; we `.remove()` them via `on_cleanup`, which — inside
    // an effect body — runs before the next re-run (when `open` flips back) and on
    // unmount, so there is no leaked listener or manual bookkeeping.
    Effect::new(move |_| {
        if !open.get() {
            return;
        }

        let close_outside = close.clone();
        let handle_outside = window_event_listener(leptos::ev::mousedown, {
            let trigger_ref = trigger_ref;
            let menu_ref = menu_ref;
            move |ev: web_sys::MouseEvent| {
                let Some(target) = ev.target() else { return };
                let el = target.unchecked_ref::<web_sys::Element>();
                let in_trigger = trigger_ref
                    .get()
                    .map(|t| t.contains(Some(el)))
                    .unwrap_or(false);
                let in_menu = menu_ref.get().map(|m| m.contains(Some(el))).unwrap_or(false);
                if !in_trigger && !in_menu {
                    close_outside.run(());
                }
            }
        });

        let close_esc = close.clone();
        let handle_esc = window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape" {
                close_esc.run(());
            }
        });

        on_cleanup(move || {
            handle_outside.remove();
            handle_esc.remove();
        });
    });

    // Body scroll-lock while open.
    effect_lock_body_scroll(open.into());

    view! {
        <div class="relative">
            <div node_ref=trigger_ref style=trigger_style on:click=move |e| toggle.run(e)>
                {trigger(open_signal, toggle.clone())}
            </div>
            <div
                node_ref=menu_ref
                class=move || {
                    tw_merge::tw_merge!(
                        "z-50 transition-all duration-200",
                        "data-[state=closed]:pointer-events-none data-[state=closed]:scale-95 data-[state=closed]:opacity-0",
                        "data-[state=open]:scale-100 data-[state=open]:opacity-100",
                        content_class.clone()
                    )
                }
                style=menu_style
                data-state=move || if open.get() { "open" } else { "closed" }
            >
                {content(open_signal, close.clone())}
            </div>
        </div>
    }
}
