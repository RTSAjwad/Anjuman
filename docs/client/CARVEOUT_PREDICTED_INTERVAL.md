# Carve-out (shell agent → core agent): surface predicted_interval on browser cards (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The browser's "Grade" action opens a dialog with Again/Hard/Good/Easy answer
buttons, styled like the study screen. The study screen shows the FSRS-predicted
interval under each button (`StudyCardView.predicted_interval`, a `rating →
seconds` map). The browser card has no such field, so the dialog currently shows
no durations.

## The gap

`predicted_interval` exists only on the study path (`StudyCard`/`StudyAdvance` →
`StudyCardView`). It is **not** on `contracts::cards::CardBrowserResponse` (nor
`CardBrowserView`), so the browser's Grade dialog can't render durations.

## What to add

### `contracts/src/cards.rs`
`CardBrowserResponse`: add the FSRS-predicted intervals for this card:
```rust
/// rating (1..=4) -> predicted interval in seconds; same shape as the study
/// screen's `StudyCard.predicted_interval`. `None` when the server returned no
/// prediction.
pub predicted_interval: Option<std::collections::HashMap<i32, i64>>,
```
(matching the `StudyCard.predicted_interval` wire shape — string keys on the wire
is fine if you prefer mirroring that; just make the client mapping consistent.)

### `server/src/handlers/card_browser.rs`
Compute it per card from the card's scheduling state (`stability`/`difficulty`/
`state`/`due_at` already selected in `browse_cards`), reusing whatever computes
`StudyCard.predicted_interval` in the study path. Populate the new field in
`rows_to_responses`. If the study path has a reusable "predict intervals" helper,
call it here rather than duplicating.

### `client/shared/src/app.rs`
`CardBrowserView`: add `pub predicted_interval: Option<HashMap<i32, i64>>`, mapped
from the new wire field (mirroring `StudyCardView::from`'s string→i32 coercion).

## Not changing

- No new events (`BrowserGradeCard { card_id, rating }` already exists).
- No change to `StudyCardView.predicted_interval`.

## Validation

- `server/tests/card_browser.rs`: a review/learning card carries a non-empty
  `predicted_interval`; a new card without a prediction carries `None`.
- `cargo test` green in `server/` + `client/shared/`; OpenAPI regenerates clean.

## Not my scope

The shell will render the durations (via the existing `formatInterval` helper)
under each Grade-dialog button once `predicted_interval` lands.
