# Carve-out (shell agent → core agent): surface note-level suspended/buried on browser cards (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The browser top bar splits into "Card Actions" and "Note Actions" groups. The note
group's Suspend/Bury are **toggles**, but the shell currently drives their pressed
state from the *selected card's* `suspended`/`buried` — a proxy, not the note's true
state. That is usually right (note suspend/bury are bulk-uniform), but wrong when a
note spans decks and only one of its cards is visible/suspended.

## Why a proxy is wrong (the exact bug)

Note suspend/bury are **uniform bulk ops** — `card_mod.rs::suspend_note`/`bury_note`
set `student_card_states.suspended`/`buried_at` identically across *all* of a note's
cards. But `CardBrowserView` only carries the **per-card** `suspended`/`buried`
(mapped from `CardBrowserResponse.suspended`/`buried_at`, which read the single
row's state). So if a note spans two decks and only one deck's card is in the
current browser page, the toggle reflects *that card* — not the note.

## What to add

Surface note-level state on `CardBrowserResponse` and map it through
`CardBrowserView`, so the note toggles read the note's true aggregate state.

### `contracts/src/cards.rs`
Add to `CardBrowserResponse`:
```rust
/// True if *all* of the note's cards are suspended for this student.
pub note_suspended: bool,
/// True if *all* of the note's cards are buried for this student.
pub note_buried: bool,
```

### `server/src/handlers/card_browser.rs`
Compute these in `rows_to_responses` per card. Because note suspend/bury are
uniform, the simplest correct value is the note's cards' shared state. Two options
(your choice — recommend (a) for correctness/simplicity):

- (a) A per-note aggregate query (e.g. `SELECT BOOL_AND(suspended), BOOL_AND(buried_at IS NOT NULL) ... WHERE note_id = $1`), reusing the existing `note_id` on the row.
- (b) Set `note_suspended = r.suspended` / `note_buried = r.buried_at.is_some()`
  (the per-card values) — correct *only* because note ops are uniform, but simpler
  and sufficient given that invariant. (This is exactly the proxy the shell is
  currently using, moved server-side.)

Given the invariant is enforced by the endpoints and documented in
`fetch_note_result_state` ("all cards are set uniformly, so reading one is
sufficient"), (b) is acceptable; (a) is more defensive if a future code path ever
sets cards non-uniformly.

### `client/shared/src/app.rs`
`CardBrowserView`: add `pub note_suspended: bool` and `pub note_buried: bool`, mapped
from the new wire fields.

## Not changing

- No new events (the four `BrowserSuspendNote`/`BrowserUnsuspendNote`/
  `BrowserBuryNote`/`BrowserUnburyNote` already exist and refresh on completion).
- No change to card-level `suspended`/`buried` (still used by the Card Actions
  toggles).

## Validation

- `server/tests/card_browser.rs`: a card whose note is fully suspended/buried
  carries `note_suspended`/`note_buried` = true; a note only partially represented
  (if you choose (a)) or uniformly unburied returns false.
- `cargo test` green in `server/` + `client/shared/`; OpenAPI regenerates clean.

## Not my scope

The shell will switch the note toggles to read `note_suspended`/`note_buried` once
they land. Nothing else.
