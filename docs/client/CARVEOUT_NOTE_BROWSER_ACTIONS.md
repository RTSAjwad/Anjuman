# Carve-out (shell agent → core agent): note-scoped suspend/bury in the browser (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The browser top bar is being split into a "Card Actions" group (Suspend/Bury/
Reschedule/Change deck/Flag — all card-scoped, already exposed) and a "Note
Actions" group (Suspend Note / Bury Note / Delete Note). Delete Note already exists
(`NoteDelete { note_id }`), but Suspend/Bury **Note** are not surfaced to the
browser.

## The gap

The core's note suspend/bury today is **study-only** and takes no id — it acts on
`model.current_card.note_id`:

```rust
Event::SuspendNote,   // -> POST /notes/{current_card.note_id}/suspend
Event::BuryNote,      // -> POST /notes/{current_card.note_id}/bury
```

The browser needs **note-id-scoped** equivalents, mirroring the existing card-scoped
browser actions (`BrowserSuspendCard`/`BrowserBuryCard`/`BrowserUnburyCard`). The
server endpoints already exist:

- `POST /notes/{note_id}/suspend`  (`card_mod::suspend_note`)
- `POST /notes/{note_id}/unsuspend` (`card_mod::unsuspend_note`)
- `POST /notes/{note_id}/bury`     (`card_mod::bury_note`)
- `POST /notes/{note_id}/unbury`   (`card_mod::unbury_note`)

## What to add (all in `client/shared/src/app.rs`)

Shell-facing events, mirroring the card-scoped `BrowserSuspendCard`/`BrowserBuryCard`
pattern (id + version), each posting to the matching `/notes/{note_id}/...` endpoint:

```rust
BrowserSuspendNote   { note_id: i64 },
BrowserUnsuspendNote { note_id: i64 },
BrowserBuryNote      { note_id: i64 },
BrowserUnburyNote    { note_id: i64 },
```

Each `then_send`s a core-local completion event (e.g. `BrowserNoteActionDone(Result<...>)`),
and on `Ok` refreshes both lists (`refresh_notes(model).and(refresh_browser(model))` —
mirroring `NoteDeleted`/`NoteTagsChanged`) so the affected cards/notes re-sync
(their `suspended`/`buried` state and, for cards, `suspended`/`buried` columns).

## Not changing

- No `contracts/` or `server/` change (endpoints + wire types exist).
- `Delete Note` is unchanged (`NoteDelete` already exists).
- Study-screen `SuspendNote`/`BuryNote` unchanged.

## Validation

- `cargo test -p shared` green; add tests analogous to the existing
  `browser_suspend_card_...` / `bury_note_emits_post` that `BrowserSuspendNote`/
  `BrowserBuryNote`/`BrowserUnburyNote` emit the right `POST /notes/{id}/...` and
  refresh both lists on success.
- `server/tests/` note-mod tests still pass (endpoints pre-existing).

## Not my scope

The shell's "Card Actions" / "Note Actions" button groups are mine; I'll wire
`BrowserSuspendNote`/`BrowserBuryNote` (and their unsuspend/unbury) once these
events land. Confirm when they're in.
