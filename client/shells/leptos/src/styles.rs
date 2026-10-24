//! Scoped, component-owned styles.
//!
//! The shell's reusable primitives should be the source of their own styling —
//! not the page, and not another component. Rust-ui/leptos-ui (and our current
//! code) style via *global* Tailwind utilities + theme tokens, which leaves a
//! residual failure mode: a parent's unlayered author CSS can leak into a
//! component's interior (list markers, margin/padding, gap, box-sizing), so the
//! same component renders differently depending on where it is mounted.
//!
//! `scoped_style` neutralizes that: it (1) deterministically derives a class
//! from a stable `tag`, (2) injects a `<style>` into `<head>` exactly once per
//! `tag`, and (3) the caller scopes its rules under that class so ancestors
//! cannot reach the component's internals. Nothing here interpolates user input.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// The set of already-injected tags, so we never add the same `<style>` twice.
static INJECTED: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

fn injected_set() -> &'static Mutex<HashSet<&'static str>> {
    INJECTED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Returns a deterministic class name for `tag` and injects `css` once.
///
/// `tag` is a stable, human-readable label (e.g. `"ui-select"`); `css` is a
/// `'static` stylesheet whose selectors are scoped under the returned class
/// (write them as `.{class} ul { … }` by formatting the class into the string,
/// or by using descendant selectors). The class is deterministic, so a component
/// can embed it into a `format!`'d rule set and still inject only once.
pub fn scoped_style(tag: &'static str, css: &'static str) -> &'static str {
    let class = intern(tag);
    {
        let set = injected_set();
        let mut guard = set.lock().expect("styles registry poisoned");
        if guard.insert(tag) {
            inject(css);
        }
    }
    class
}

/// A stable `'static` class name derived from `tag` (no user input).
fn intern(tag: &'static str) -> &'static str {
    // The class is deterministic per tag (used in the caller's CSS selectors), so
    // it must not contain a random suffix. Sanitize `tag` (it is a code literal,
    // but be defensive) and prefix.
    let cleaned: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    Box::leak(format!("anj-{cleaned}").into_boxed_str())
}

/// Inject a `<style data-anjuman-scoped>` into `<head>`.
fn inject(css: &'static str) {
    let document = leptos::prelude::document();
    let Some(head) = document.head() else {
        return;
    };
    let Ok(style) = document.create_element("style") else {
        return;
    };
    let _ = style.set_attribute("data-anjuman-scoped", "");
    // The caller-scoped CSS is safe: nothing here interpolates user data.
    style.set_text_content(Some(css));
    let _ = head.append_child(&style);
}
