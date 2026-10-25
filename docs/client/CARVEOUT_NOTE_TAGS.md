# Carve-out (shell agent → core agent): note-tag management + tags on browser cards (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The shell is adding tag management to the unified note-details panel (the panel
shared by the Cards and Notes browser views). UX is settled:

- Tags render/​edit via a **self-built chip input** in the note panel.
- **Immediate save** — adding/removing a tag is its own round-trip, *not* part of
  the field-save button (mirrors Anki's tag editor).
- **Autocomplete** from the existing global tag namespace.
- **Free-form** — new tags are allowed (the server already supports this via
  `ensure_tag`).

The server + contracts already expose every mutation endpoint the UI needs; the
**core** (`client/shared/`) is the only gap on the mutation side. A second gap:
tags are note-level, but the card-browser path cannot currently *see* them —
`CardBrowserResponse` (wire) and `CardBrowserView` (shell) carry no `tags`, and
the `GET /cards` endpoint never joins `note_tags`.

This carve-out spans the core agent's full write scope: `contracts/`, `server/`,
and `client/shared/`.

## 1. Contracts (`contracts/src/cards.rs`)

Add the note's tags to `CardBrowserResponse`, matching `NoteResponse.tags`:

```rust
pub tags: Vec<Tag>,   // note_tags joined; order by name
```

(`Tag` from `anjuman_contracts::tags`, already the type `NoteResponse.tags` uses
— import it in `cards.rs`.)

## 2. Server (`server/src/handlers/card_browser.rs`, `server/src/tags.rs`)

Populate `tags` for each browser card. The handler already does per-card N+1
lookups (`note_types::get_note_type`) inside `rows_to_responses`, so the
idiomatic change is a per-note `tags::tags_for_note(&db, note_id)` call there
(`server/src/tags.rs::tags_for_note` already exists and returns `Vec<Tag>`
ordered by name). Populate `tags` on each `CardBrowserResponse`.

No new route is needed — this is an additive field on the existing `GET /cards`
response.

## 3. Core (`client/shared/src/app.rs`)

### 3a. Surface tags on the card view

- Add `pub tags: Vec<String>` to `CardBrowserView` (flatten `Tag.name`, matching
  how `NoteView::from` flattens `NoteResponse.tags` → `Vec<String>`).
- Map it in `From<&CardBrowserResponse> for CardBrowserView`.

### 3b. New mutation events

Add two shell-facing events (mirror the existing `BrowserSuspendCard`/`NoteSave`
style — discrete, immediate, id + payload):

```rust
/// Add tags to a note (free-form names; server normalises + dedups).
NoteAddTags { note_id: i64, tags: Vec<String> },
/// Remove tags from a note (by name).
NoteRemoveTags { note_id: i64, tags: Vec<String> },
```

Wire them to the existing endpoints (`server/src/routes.rs` already has them):

- `NoteAddTags` → `POST /notes/{note_id}/tags` body `AddTags { tags }`
- `NoteRemoveTags` → `DELETE /notes/{note_id}/tags` body `RemoveTags { tags }`

Both return the note's refreshed `Vec<Tag>` (the handlers already do). Add the
corresponding core-local completion events (e.g. `NoteTagsChanged(Result<...>)`),
gated `#[serde(skip)]`/`#[facet(skip)]` like the other `*Result` events.

### 3c. On completion, re-sync the selected note/card (so the chip input updates
      reactively — no shell-side tag mirroring)

After a successful add/remove, the selected note's `tags` must be re-synced into
both selection slots, mirroring the existing `resync_notes_selected` /
`resync_browser_selected` pattern:

- Re-sync `notes_selected` from the refreshed tags (the endpoint response already
  carries the full note tag list).
- Re-sync `browser_selected`'s note tags in cards mode. Because `browser_selected`
  is a *card* (cards don't own tags), refresh the card browser via
  `refresh_browser(model)` after a tag change **iff** the panel is in cards mode —
  that re-fetches `GET /cards` (now with `tags` from §1/§2) and `resync_browser_selected`
  picks up the fresh tags. (If cards mode isn't active/selected, a notes-only
  re-sync suffices — your call on the minimal correct refresh; the goal is the
  chip input shows the freshest tags immediately.)

Also: `TagsRequested`/`TagTreeRequested` and `ViewModel.tags`/`tag_tree` already
exist — unchanged; the shell will fire `TagsRequested` when the browser opens for
the autocomplete namespace.

## Not needed (no change)

- No new mutation endpoints (server has them).
- No `NoteCreate`/`NoteDelete` change.

## Validation (DoD)

- Server integration test in `server/tests/card_browser.rs` (or `tags.rs`):
  `GET /cards` now returns each card's `tags` (ordered by name) — create a note
  with tags, fetch cards, assert `tags` present.
- Shared test(s): `CardBrowserView` carries `tags`; `NoteAddTags` /
  `NoteRemoveTags` emit the correct HTTP `POST`/`DELETE` with the right body and
  re-sync the selection on completion.
- `cargo test` green in `server/` and `client/shared/`; OpenAPI still generates
  (`cargo build --features openapi` in `contracts/`).

## Not my scope

The chip-input component (`client/shells/react`) is mine; I'll build it against
`tags: string[]` + `onAdd`/`onRemove`, then wire to `NoteAddTags`/`NoteRemoveTags`
once this lands. Please confirm when `CardBrowserView.tags` + the two events are
in.
