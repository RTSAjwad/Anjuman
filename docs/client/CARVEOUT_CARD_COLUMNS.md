# Carve-out (shell agent → core agent): card-browser columns need sort_field, template_name, due_at, deck path (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The card browser table is being reworked to these columns (in order):

1. Sort Field
2. Card Type (= **template name**)
3. Due Date
4. State
5. Flag
6. Deck — full ancestor path joined with `::` (e.g. `Biology 101::Cell Biology`), not the stripped single-deck title

Of these, **State** (`CardBrowserView.state`) and **Flag** (`CardBrowserView.flag`)
are already exposed. The other four are not — they need `CardBrowserResponse`
(wire) and/or `CardBrowserView` (shell) changes, and one needs a new
server-side computation.

## What's already on the wire but dropped from the shell view

`contracts::cards::CardBrowserResponse` already has:

- `template_name: String` — dropped by `CardBrowserView::from`.
- `due_at: Option<DateTime<Utc>>` — dropped by `CardBrowserView::from`.

## What's genuinely new / needs computation

1. **`sort_field`** — the note type's resolved sort field value (Anki's "Sort
   Field"), exactly like `NoteResponse.sort_field`. The server's
   `notes.rs::fetch_note_with_cards` already resolves this via
   `nt.effective_sort_field()` + `fields.get(...)`; the card browser
   (`card_browser.rs::rows_to_responses`) already fetches `nt` (a full `NoteType`)
   and `fields`, so it can resolve the same value with no new query. Add
   `sort_field: Option<String>` to `CardBrowserResponse` and `CardBrowserView`.

2. **`deck_path`** — the full ancestor chain of the card's deck, top-down, joined
   with `::` (e.g. `Biology 101::Cell Biology`). Currently `deck_title` is only
   the immediate `d.title`; there is no ancestor walk. Needed: a server-side walk
   over `decks.parent_id` (a self-FK — see migration `0001_create_schema.sql`),
   producing the `::`-joined path. Add `deck_path: String` to
   `CardBrowserResponse` and `CardBrowserView` (keep `deck_title` for the
   filter/tree; `deck_path` is the display column).

## Design decision for `due_at` (please follow)

The core's shell-facing `*View` structs do **not** carry `DateTime` today —
`NoteView`, `StudyCardView`, and `CardBrowserView` all deliberately drop timestamps
(the Facet types avoid `chrono::DateTime`). Please surface a **pre-formatted
string** rather than raw `DateTime`:

- Add `due_at: Option<String>` to `CardBrowserView` only (no wire change needed —
  `CardBrowserResponse.due_at` already exists as `Option<DateTime<Utc>>`), mapping
  via `DateTime::to_rfc3339()` (or a compact `%Y-%m-%d %H:%M`). Recommend
  `%Y-%m-%d %H:%M` (UTC) for a stable, locale-agnostic display the shell renders
  verbatim — the shell must **not** do date formatting.

If you'd rather keep `DateTime` out of the shell entirely and handle formatting
elsewhere, flag it — but a `String` mapping in `CardBrowserView::from` is the
simplest and matches the existing "no `DateTime` in `*View`" convention.

## Changes requested (summary)

### `contracts/src/cards.rs`
- `CardBrowserResponse`: add `sort_field: Option<String>` and
  `deck_path: String`.

### `server/src/handlers/card_browser.rs`
- In `rows_to_responses`: populate `sort_field` (via
  `nt.effective_sort_field()` + `fields.get(...)`, mirroring
  `notes.rs::fetch_note_with_cards`) and `deck_path` (ancestor walk over
  `decks.parent_id`).
- No new query needed for `sort_field` (already have `nt` + `fields`); `deck_path`
  needs a recursive parent lookup (batch it to avoid N+1, or add a helper in
  `server/src/decks.rs` if a deck-title-map already exists there).

### `client/shared/src/app.rs`
- `CardBrowserView`: add `sort_field: Option<String>`, `template_name: String`,
  `due_at: Option<String>` (formatted), `deck_path: String`.
- Map all four in `From<&CardBrowserResponse>`.

## Not changing

- State + Flag already present.
- No new events/endpoints.

## Validation (DoD)

- Server test(s) in `server/tests/card_browser.rs`: a card's response carries the
  resolved `sort_field`, `deck_path` (nested deck → `::` chain), and `due_at`.
- Shared test(s): `CardBrowserView` maps `sort_field`/`template_name`/`due_at`/
  `deck_path` through (analogous to `note_view_carries_sort_field`).
- `cargo test` green in `server/` + `client/shared/`; OpenAPI regenerates clean.

## Not my scope

The shell table columns are mine; I'll wire them against `sort_field`,
`template_name`, `due_at`, `deck_path` (and existing `state`/`flag`) once this
lands. Please confirm when `CardBrowserView` carries those four fields.
