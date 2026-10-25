# Carve-out (shell agent → core agent): NoteCreated should also refresh the browser (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Symptom

Creating a note (the shell's "Add note" dialog → `NoteCreate`) succeeds, but the
new note's cards don't appear in the **Cards** browser until the user switches
Cards ↔ Notes (which re-fires `CardsRequested`). Toggling views is the only way
to see the new card(s).

## Root cause

`NoteCreated(Ok)` refreshes **only the notes list**:

```rust
Event::NoteCreated(Ok(_)) => {
    model.notes_error = None;
    refresh_notes(model)   // ← notes only
}
```

Creating a note also generates its cards (`sync_card_rows` in the server), so the
Cards view should reflect them too. This is inconsistent with the directly-parallel
`NoteTagsChanged(Ok)` handler, which refreshes **both**:

```rust
Event::NoteTagsChanged(Ok(_)) => {
    model.notes_error = None;
    refresh_notes(model).and(refresh_browser(model))   // both
}
```

## What to do (all in `client/shared/src/app.rs`)

Change `NoteCreated(Ok)` to refresh both, mirroring `NoteTagsChanged`:

```rust
Event::NoteCreated(Ok(_)) => {
    model.notes_error = None;
    refresh_notes(model).and(refresh_browser(model))
}
```

(`refresh_browser` re-fetches `GET /cards` using the stored `browser_filter`; it
returns `render::render()` when no filter is recorded yet, so this is safe even
before the browser has been opened once.)

## Consider (adjacent, optional — flag if you disagree)

`NoteSaved(Ok)` and `NoteDeleted(Ok)` currently also only call `refresh_notes`.
Saving a note regenerates its cards (decks/fields can change), and deleting removes
them — so those likely want `refresh_browser` too for the same no-toggle
consistency. This carve-out asks **only** for `NoteCreated` (the reported bug);
please confirm whether you want to extend the same fix to `NoteSaved`/`NoteDeleted`
now or leave them for a follow-up.

## Not changing

No `contracts/`/`server/` change; no new event.

## Validation

- `cargo test -p shared` green; add/extend a test that `NoteCreated(Ok)` emits both
  a notes refresh and a browser refresh (analogous to `note_tags_changed` if such a
  test exists).
- Manual: create a note while on the Cards view → its card appears without toggling.

## Not my scope

The shell is correct (it maps `NoteCreate` and re-renders from
`browser_cards`/`notes`); no shell change needed once this lands.
