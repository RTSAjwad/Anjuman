# Carve-out (shell agent → core agent): study-screen "Reset Card" + note actions (core scope)

**From:** shell agent
**To:** core agent
**Status:** resolved (commit `099cda4`)

## Context

The study screen's "⋮" dropdown is being reorganised into two sections:

- **Card Actions** — Bury Card, Suspend Card, Reschedule, and a new **Reset Card**
  (Anki "Forget").
- **Note Actions** — Bury Note, Suspend Note, and (requested) **Duplicate Note** +
  **Delete Note**.

The re-section, label renames, and the existing Bury/Suspend/Reschedule card+note
items are **shell-only** and already done. The three *new* items needed core
support, now implemented.

## Resulting core surface (consume this)

All changes are in `client/shared/src/app.rs` (no `contracts/`/`server/` change).

### 1. Reset Card (study context) — done

- `Event::ResetCard` (unit) → `POST /cards/{card_id}/reset` using the current
  card id. No-op when `current_card` is `None`.
- `Event::ResetCardDone(Result<Response<CardModResponse>>)` → on `Ok`, re-fetches
  the study loop (`StartStudy`), so the reset card (now "new"/no longer due)
  advances correctly. On `Err`, sets `card_action_error`.

### 2. Duplicate Note — done via a note fetch (NOT a `StudyCard` enrichment)

**Decision:** the original "no wire DTO change via recommendation (b)" assumption
was **wrong** — `StudyCard` (the wire type) carries `card_id`, `note_id`, `front`,
`back`, `styling`, and scheduling fields only; it has **no** `note_type_id`,
`fields`, `tags`, or `deck_id`, so those cannot be surfaced on `StudyCardView`
without a contracts + server change.

Instead, the core exposes a fetch of the full note:

- `Event::NoteDetailRequested { note_id }` → `GET /notes/{note_id}`.
- `Event::NoteDetailResult(Result<Response<NoteResponse>>)` → populates
  `ViewModel.study_note: Option<NoteView>`.

`NoteView` already carries `note_type_id`, `note_type_name`, `fields`
(`HashMap<String,String>`), `tags: Vec<String>`, and `deck_id: Option<i64>` — the
full prefill payload. The shell fires `NoteDetailRequested { note_id: card.note_id }`
when the user picks "Duplicate Note", then prefills its existing `AddNoteDialog`
from `view.study_note`.

### 3. Delete Note (study context) — done (advance/close)

- `Event::NoteDelete { note_id }` now records `Model.pending_note_delete`.
- `Event::NoteDeleted(Ok)` detects whether the deleted note was the in-study note;
  if so it clears `current_card` + `study_note` and chains `StartStudy` (advance or
  close the study loop — if the deck is now empty, `StartStudy` returns
  `next_card == None`, so the shell shows "nothing due"). It also still refreshes
  the browser lists as before.

---

## Reset paths

`study_note` and `pending_note_delete` are cleared on logout and on
`clear_deck_selection` (leaving study / opening a deck resets them).

## Tests

Named per criterion, in `client/shared/src/app.rs`:

- `reset_card_emits_post`
- `reset_card_no_current_card_is_noop`
- `reset_card_done_refreshes_study`
- `note_detail_requested_emits_get`
- `note_detail_result_populates_study_note`
- `note_delete_studied_note_advances_study`

## Not changing

- No `contracts/` change (the reset endpoint already exists; `NoteResponse` already
  carries the duplicate-prefill fields).
- No `server/` change (reset + note fetch/delete endpoints already exist).

## Open items / notes for the shell

- Duplicate Note is fetched on demand (`NoteDetailRequested`) rather than being
  pre-attached to `StudyCardView`. If you'd prefer the note fields attached
  directly to the study card (one fewer round-trip), that is a follow-up that
  *does* cross `contracts/` + `server/` — file a new carve-out if you want it.
- The shell should fire `NoteDetailRequested` only when the "Duplicate Note"
  action is invoked (not on every study card render), to avoid a redundant fetch.
