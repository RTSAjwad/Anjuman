# Carve-out (shell agent → core agent): fix `note_suspended`/`note_buried` — per-card proxy is wrong (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending (correctness bug in `1ceb0a6`)

## Symptom

Suspend **one card** (the browser's "Suspend Card", which is a per-card op), and the
"Suspend Note" toggle lights up — even when the note's *other* cards remain
unsuspended.

## Root cause

`1ceb0a6` implemented `note_suspended`/`note_buried` as **option (b)** — copying the
per-card `r.suspended`/`r.buried_at`, justified by the "uniform bulk-op invariant":

```rust
note_suspended: r.suspended.unwrap_or(false),
note_buried: r.buried_at.is_some(),
```

But that invariant only holds for **note-level** operations (`suspend_note`/`bury_note`,
which set *every* card of the note identically). It does **not** hold for the
**per-card** `suspend`/`bury` operations (`card_mod.rs::suspend`/`bury`), which set a
*single* card's `student_card_states` row. So `r.suspended` is the *selected card's*
state, not the note's — exactly the proxy bug the carve-out warned about.

## What to fix (`server/src/handlers/card_browser.rs`)

Compute `note_suspended`/`note_buried` as a true note-level aggregate over **all** of
the note's cards (for the student), not the single row. In `rows_to_responses`, for
each card row (which already carries `r.note_id` and `r.deck_id`), replace the
per-card copy with an aggregate query, e.g.:

```sql
SELECT
  BOOL_AND(suspended)                     AS note_suspended,
  BOOL_AND(buried_at IS NOT NULL)         AS note_buried
FROM student_card_states scs
JOIN cards c ON c.id = scs.card_id
WHERE scs.student_id = $1 AND c.note_id = $2
```

Using `AND` means "all of the note's cards are suspended/buried", matching the note
toggle semantics ("Suspend Note" as a pressed state = the whole note is suspended).

> Note: because this is per-row in the existing per-card N+1 loop, consider whether a
> batched note-level lookup is warranted (many duplicate `note_id`s per page). A
> per-note memo (or a single `LEFT JOIN` aggregate in the main `GET /cards` query) is
> preferable for performance; correctness first, then avoid the obvious N-per-note
> repeat if straightforward.

## Not changing

- Keep the contract fields and `CardBrowserView` mapping (already correct).
- No event change.
- Card-level `suspended`/`buried` stay per-card (the Card Actions toggles depend on
  them).

## Validation

- `server/tests/card_browser.rs`: add a case where **one card of a note is suspended
  but a sibling isn't** → `note_suspended` is `false` (while that card's own
  `suspended` is `true`); and a note fully suspended → `note_suspended` is `true`.
- `cargo test` green in `server/`.

## Not my scope

The shell reads `note_suspended`/`note_buried` directly; no shell change needed once
this is fixed.
