//! Table — a composable table, styled to match `rust-ui/leptos-ui`'s `Table`
//! family. Compose `UiTable` with `UiTableHeader`/`UiTableBody`/`UiTableHead`/
//! `UiTableRow`/`UiTableCell`.

use leptos::prelude::*;

/// The `<table>`, wrapped in our scroll container (`.card-browser__table`).
#[component]
pub fn UiTable(children: Children) -> impl IntoView {
    view! {
        <div class="card-browser__table">
            <table class="w-full text-sm">
                {children()}
            </table>
        </div>
    }
}

/// The `<thead>`, with a bottom border and muted background.
#[component]
pub fn UiTableHeader(children: Children) -> impl IntoView {
    view! {
        <thead class="[&_tr]:border-b bg-muted">
            {children()}
        </thead>
    }
}

/// The `<tbody>` (last row has no bottom border).
#[component]
pub fn UiTableBody(children: Children) -> impl IntoView {
    view! {
        <tbody class="[&_tr:last-child]:border-0">
            {children()}
        </tbody>
    }
}

/// A header cell.
#[component]
pub fn UiTableHead(children: Children) -> impl IntoView {
    view! {
        <th class="h-10 px-3 text-left align-middle font-medium text-muted-foreground">
            {children()}
        </th>
    }
}

/// A data row, with a bottom border, hover state, and optional `selected`
/// highlight + `on_click` (for selectable rows).
#[component]
pub fn UiTableRow(
    children: Children,
    #[prop(optional)] selected: Signal<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <tr
            class="cursor-pointer border-b transition-colors hover:bg-muted/50"
            class:bg-muted=selected
            on:click=move |_| if let Some(cb) = on_click.as_ref() { cb.run(()) }
        >
            {children()}
        </tr>
    }
}

/// A data cell.
#[component]
pub fn UiTableCell(children: Children) -> impl IntoView {
    view! {
        <td class="px-3 py-2 align-middle">
            {children()}
        </td>
    }
}
