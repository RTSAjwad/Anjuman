# Carve-out (shell agent → core agent): note-level suspend/bury state on NoteView (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Symptom

In the **Notes** browser, "Suspend Note" / "Bury Note" currently render as one-way
buttons (not toggles) because `NoteView` carries no `note_suspended`/`note_buried`.
The user wants them to be **toggles that reflect state**, matching the Cards
browser — but the state isn't surfaced for the notes list.

## What already exists

- `CardBrowserView` already has `note_suspended`/`note_buried` (card-browser path,
  computed per-student via `student_card_states`).
- The four note browser events (`BrowserSuspendNote`/`BrowserUnsuspendNote`/
  `BrowserBuryNote`/`BrowserUnburyNote`) exist and post to
  `POST /notes/{id}/suspend|…`, which operate on the **authenticated user's own**
  `student_card_states` (`claims.sub`).

## The gap (and an important semantic wrinkle)

1. `NoteView` (Notes browser) has no `note_suspended`/`note_buried`, so the shell
   can't render pressed-state toggles there.

2. **Semantic wrinkle to resolve:** the card browser is naturally student-scoped
   (`student_card_states` per `student_id`), but the Notes browser
   (`GET /notes` → `list_notes`) is a **teacher/admin management view** that lists
   notes *globally*, not per-student — and `fetch_note_with_cards` only takes
   `note_id` (no `student_id`). So "this note is suspended/buried" has no
   well-defined answer in the notes list without choosing *whose* scheduling state
   to read.

   Please decide and document the meaning of `note_suspended`/`note_buried` in the
   Notes browser. Recommended: read the **authenticated user's own** state
   (`claims.sub`) — the same user the `POST /notes/{id}/suspend` acts on — so the
   toggle's action and its displayed state are consistent with each other. (For a
   teacher viewing another student's cards this may be "their own" rather than
   "the student's", but that's the only self-consistent definition given the
   existing endpoints.)

## What to add

### `contracts/src/notes.rs`
`NoteResponse`: add
```rust
pub note_suspended: bool,
pub note_buried: bool,
```

### `server/src/handlers/notes.rs`
`fetch_note_with_cards` must gain a `student_id` parameter (or a sibling helper)
and compute the note-level aggregate from `student_card_states`, mirroring
`card_browser.rs::note_state_for`:

```sql
SELECT BOOL_AND(scs.suspended), BOOL_AND(scs.buried_at IS NOT NULL)
FROM student_card_states scs JOIN cards c ON c.id = scs.card_id
WHERE scs.student_id = $1 AND c.note_id = $2
```

Thread the authenticated `claims.sub` through every caller of
`fetch_note_with_cards` (`list_notes`, `get_note`, `create_note`, `update_note`,
and `create_note`'s final re-fetch).

### `client/shared/src/app.rs`
`NoteView`: add `pub note_suspended: bool` and `pub note_buried: bool`, mapped from
the new wire fields.

## Not changing

- No new events (the four browser note events exist).
- `CardBrowserView.note_suspended`/`note_buried` unchanged.

## Validation

- `server/tests/`: a note whose cards are all suspended carries
  `note_suspended=true` on `GET /notes`; a partially/not-suspended note is `false`.
- `cargo test` green in `server/` + `client/shared/`; OpenAPI regenerates clean.

## Not my scope

The shell will switch the Notes-route note actions to real `Toggle`s reading
`note_suspended`/`note_buried` once they land. Nothing else.
