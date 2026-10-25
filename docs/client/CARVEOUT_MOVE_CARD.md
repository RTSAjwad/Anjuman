# Carve-out (shell agent → core agent): move a browser card to another deck (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The card browser's top bar (suspend/bury/flag/reschedule) needs a new "Change
deck" action: move the **selected** card to a different deck. The server already
exposes the endpoint; the **core** does not surface it to the shell.

## What already exists

- `contracts::cards::MoveCardBody { deck_id: i64 }` and `MoveCardResponse { card_id,
  deck_id }`.
- `PATCH /cards/{card_id}/deck` in `server/src/handlers/card_mod.rs::move_card`
  (target-deck validation + collaborator checks already done server-side).
- `ViewModel.decks: Vec<DeckSummary>` (already carries `id` + `title` + nested
  `children`) — the shell can render the deck picker from this, no new field needed.
- The core's existing browser card actions (`BrowserSuspendCard`,
  `BrowserRescheduleCard`, etc.) already follow the pattern to mirror: emit an
  HTTP request on the card id, then refresh the browser + re-sync the selection.

## What's missing (the gap)

There is **no core event** for moving a card. `client/shared/src/app.rs` has no
`MoveCard`/`BrowserMoveCard`/`MoveCardResult` variant.

## What to add (all in `client/shared/src/app.rs`)

1. A shell-facing event, mirroring `BrowserRescheduleCard`:

   ```rust
   /// Move a browser card (by id) to another deck.
   BrowserMoveCard { card_id: i64, deck_id: i64 },
   ```

2. Wire it to `PATCH /cards/{card_id}/deck` with body `MoveCardBody { deck_id }`
   (auth-gated like the other browser actions), `then_send` a new core-local
   completion event, e.g.:

   ```rust
   #[serde(skip)]
   #[facet(skip)]
   BrowserMoveCardDone(#[facet(opaque)] crux_http::Result<crux_http::Response<MoveCardResponse>>),
   ```

   On `Ok`, refresh the browser (`refresh_browser(model)` — the existing helper,
   which re-fetches `GET /cards` and re-syncs `browser_selected` so the selected
   card's `deck_id`/`deck_title`/`deck_path` update). On `Err`, set
   `model.browser_error`.

## Not changing

- No `contracts/` change (wire types + endpoint exist).
- No `server/` change (endpoint exists).
- The deck picker list is the shell's to build from the existing `view.decks`.

## Validation (DoD)

- `cargo test -p shared` green, including a new test that `BrowserMoveCard`
  emits `PATCH .../cards/{id}/deck` with the correct JSON body and re-syncs on
  success (analogous to `browser_reschedule_card_...` if one exists, else
  `note_save_emits_patch_with_body` style).
- Optionally a `server/tests/` case confirming `move_card` still works (only if
  you want a server-side regression guard; the endpoint is pre-existing and tested).

## Not my scope

The shell "Change deck" button + deck-picker dialog is mine; I'll wire it to
`BrowserMoveCard { card_id, deck_id }` once the event lands. Please confirm when
it's in.
