# Carve-out (shell agent → core agent): study-screen "Reset Card" + note actions (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The study screen's "⋮" dropdown is being reorganised into two sections:

- **Card Actions** — Bury Card, Suspend Card, Reschedule, and a new **Reset Card**
  (Anki "Forget").
- **Note Actions** — Bury Note, Suspend Note, and (requested) **Duplicate Note** +
  **Delete Note**.

The re-section, label renames, and the existing Bury/Suspend/Reschedule card+note
items are **shell-only** and already done. The three *new* items below need core
support the shell does not have.

## What already exists

- `POST /cards/{card_id}/reset` — `server/src/handlers/card_mod.rs::reset` (resets
  a card's scheduling back to "new", preserving flag + row).
- `Event::BrowserResetCard { card_id }` — browser-path only; its completion
  `BrowserResetDone` refreshes the **browser**, not the study loop.
- `Event::NoteDelete { note_id }` — generic; but its completion `NoteDeleted`
  refreshes **browser lists**, not the in-flight study session.
- `StudyCardView` already carries `card_id`, `note_id`, `front`, `back`, `state`,
  `step_index`, `flag`, `suspended`, `buried` (so the shell can pass ids and render
  current card/note toggle state).

## The gaps

### 1. Reset Card (study context)

No study-context reset event. `BrowserResetCard` is wrong here: it completes via
`BrowserResetDone`, which refreshes the browser rather than re-fetching the study
loop's current card. A study-screen "Reset Card" must reset the card and then
advance/refresh the study session consistently (the card becomes "new" and is likely
no longer the due card).

**Ask:** add `Event::ResetCard` (unit; operates on `model.current_card.card_id`
exactly like the existing `BuryCard`/`SuspendCard`), wired to
`POST /cards/{card_id}/reset`, with a completion (e.g. `ResetCardDone`) that
re-fetches the study card/state (mirror how the other study card actions refresh —
please reuse whatever `BuryCard`/`SuspendCard` do on completion so the study loop
advances/re-renders correctly).

### 2. Duplicate Note

No study-context duplicate, and `StudyCardView` lacks the note fields needed to
prefill a create dialog (`note_type_id`, fields, tags, deck). Duplicate Note means
"open the create-note dialog prefilled with the current note's fields/note-type/
deck/tags".

**Ask:** either
- (a) expose `Event::NoteDuplicate { note_id }` that the shell fires, and the core
  opens/re-populates the create-note path (but the create dialog itself is shell
  state, so this doesn't cleanly map), or more simply
- (b) surface the current card's full note (`note_type_id`, `fields`, `tags`,
  `deck_id`) on `StudyCardView` (or a new `study_note` ViewModel field) so the shell
  can reuse its existing `AddNoteDialog` prefill logic (which already handles
  duplicate in the browser via `NoteView` fields/tags/deck).

**Recommendation:** (b) — add the note fields the shell's existing duplicate-prefill
needs, so no new event is required; the shell reuses its current duplicate flow.

### 3. Delete Note (study context)

`Event::NoteDelete { note_id }` exists, but `NoteDeleted` refreshes browser lists
only — it does not handle the case where the deleted note is the one currently being
studied. If the user deletes the in-study note, the study screen must advance to the
next card (or close) rather than leave a dangling current card.

**Ask:** decide the semantics and wire them. Recommended: when `NoteDelete` targets
`model.current_card.note_id`, after `NoteDeleted(Ok)` re-fetch the study card (or
`CloseDeck` if the deck is now empty). If simpler and acceptable, restrict Delete
Note to the browser only and document that study-context delete is out of scope —
but the requester asked for it in the study dropdown, so prefer wiring it.

## Not changing

- No `contracts/` change expected for `ResetCard` (endpoint exists). Duplicate may
  need an `StudyCardView` field addition (core-local `ViewModel`/`StudyCardView`);
  no wire DTO change if using recommendation (b).
- No `server/` change (reset endpoint exists; note delete exists).

## Validation (DoD)

- `cargo test -p shared` green, with tests named after each criterion, e.g.:
  - `reset_card_emits_post_to_cards_reset`
  - `study_note_delete_advances_or_closes` (or `note_delete_refreshes_study`, per
    the chosen semantics)
  - (if duplicate via (b)) `study_card_carries_note_prefill_fields`
- OpenAPI still generates (no new wire DTO unless you add one).

## Not my scope

The shell's re-section (Card Actions / Note Actions) + label renames is already
done. I'll wire the three new items once the core surface lands. Please confirm
which option you choose for Duplicate Note (a vs. b) and the Delete-Note semantics.
