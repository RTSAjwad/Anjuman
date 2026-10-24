//! Table — a composable table, `rust-ui/leptos-ui`'s `Table` family with
//! **verbatim** class strings. Compose `Table` with `TableHeader`/`TableBody`/
//! `TableHead`/`TableRow`/`TableCell`.
//!
//! ⚠️ **Divergence to revisit (docs §10.6):** rust-ui's `TableRow` is
//! presentational-only (`data-[state=selected]:bg-muted` with no interaction).
//! Our browser rows are **selectable**, so we keep `TableRow`'s `selected` and
//! `on_click` controlled props — `selected` drives `data-state=selected` (the
//! verbatim selection class) and `on_click` emits row selection. This is a
//! shell-local interactivity addition, not a class-string change.

use leptos::prelude::*;

/// The `<table>`, rust-ui's `Table` (verbatim). The scroll container is the
/// caller's layout concern (`card-browser__table` in the browser view), not the
/// component's.
#[component]
pub fn Table(children: Children) -> impl IntoView {
    view! {
        <table class="w-full max-w-7xl text-sm caption-bottom">
            {children()}
        </table>
    }
}

/// The `<thead>` (verbatim `TableHeader`).
#[component]
pub fn TableHeader(children: Children) -> impl IntoView {
    view! {
        <thead class="[&_tr]:border-b sticky top-0 z-10 bg-card">
            {children()}
        </thead>
    }
}

/// The `<tbody>` (verbatim `TableBody`).
#[component]
pub fn TableBody(children: Children) -> impl IntoView {
    view! {
        <tbody class="[&_tr:last-child]:border-0">
            {children()}
        </tbody>
    }
}

/// A header cell (verbatim `TableHead`).
#[component]
pub fn TableHead(children: Children) -> impl IntoView {
    view! {
        <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]">
            {children()}
        </th>
    }
}

/// A data row (verbatim `TableRow` class string, plus controlled selection —
/// see module doc).
#[component]
pub fn TableRow(
    children: Children,
    #[prop(optional)] selected: Signal<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <tr
            class="border-b transition-colors hover:bg-muted/50"
            data-state=move || if selected.get() { "selected" } else { "" }
            on:click=move |_| if let Some(cb) = on_click.as_ref() { cb.run(()) }
        >
            {children()}
        </tr>
    }
}

/// A data cell (verbatim `TableCell`).
#[component]
pub fn TableCell(children: Children) -> impl IntoView {
    view! {
        <td class="p-4 align-middle [&:has([role=checkbox])]:pr-0 [&:has([role=checkbox])]:pl-3">
            {children()}
        </td>
    }
}
