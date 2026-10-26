# Carve-out (shell agent → core agent): duplicate-note support — NoteCreate tags + NoteView deck (core scope)

**From:** shell agent
**To:** core agent
**Status:** done

## Context

The shell is adding a **Duplicate Note** action. Clicking it opens the existing
create-note dialog prefilled with the selected note's fields, note type, deck, and
tags. This needs two core-side gaps filled:

1. The create dialog must submit **tags** with the new note (currently `NoteCreate`
   accepts no tags; tags are only added post-hoc via `NoteAddTags`).
2. In the **Notes** browser, the shell needs the selected note's **deck** to prefill
   the create dialog (currently `NoteView` carries no deck).

## Decision (settled with the coordinator)

- For multi-deck notes, the "duplicate deck" is the **first card's deck**
  (`NoteResponse.cards[0].deck_id`, which already exists on the wire).
- Tags are carried by extending `NoteCreate` (single create round-trip), not
  create-then-add.

## 1. `NoteCreate` accepts tags (contracts + server + core)

### `contracts/src/notes.rs`
Add a `tags` field to `CreateNote` (names, matching `AddTags`):
```rust
pub struct CreateNote {
    pub note_type_id: i64,
    pub fields: Map<String, Value>,
    pub deck_id: i64,
    /// Tags to attach to the new note (free-form names; server normalises +
    /// dedups, mirroring POST /notes/{id}/tags).
    #[serde(default)]
    pub tags: Vec<String>,
}
```
(`#[serde(default)]` so older clients / existing call sites without tags keep
working.)

### `server/src/handlers/notes.rs` — `create_note`
After inserting the note + generating cards (existing `sync_card_rows`), attach the
requested tags **in the same transaction**, reusing the existing tag helpers
(`tags::normalize_name`/`tags::ensure_tag` + the `note_tags` insert) exactly as
`handlers/tags.rs::add_tags` does. Dedup + reject/ignore empty names the same way.

### `client/shared/src/app.rs`
`Event::NoteCreate` gains `tags: Vec<String>`; the core passes it through to the
wire `CreateNote`.

## 2. Surface the note's deck on `NoteView` (core)

### `client/shared/src/app.rs`
Add to `NoteView`:
```rust
/// The note's deck for "duplicate" prefill: the first card's deck, or `None` if
/// the note has no cards.
pub deck_id: Option<i64>,
```
Map it in `From<&NoteResponse>` via `n.cards.first().map(|c| c.deck_id)`.

(No `contracts`/`server` change needed for this — `NoteResponse.cards[].deck_id`
already exists.)

## Not changing

- No new event for "duplicate" itself — duplicate is shell-side prefill of the
  existing `NoteCreate`.
- `AddTags`/`NoteAddTags` unchanged.
- `CardBrowserView` already carries `deck_id` (cards mode prefill is covered).

## Validation (DoD)

- `server/tests/notes_browser.rs` (or `notes.rs`): creating a note with `tags`
  attaches them; `GET /notes` returns them; multi-deck notes surface `deck_id` =
  first card's deck.
- `cargo test` green in `server/` + `client/shared/`; OpenAPI regenerates clean.

## Not my scope

The shell's Duplicate button + prefilled create dialog are mine; I'll consume
`NoteCreate { …, tags }` and `NoteView.deck_id` once they land.
