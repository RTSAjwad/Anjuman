# Carve-out (core agent → shell agent): move deck from the deck list

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

Users need to move a deck under a new parent (or to the root) from the deck-list
screen. The server endpoint and the core surface are already done — this is shell
UI work only.

## What already exists (consume this — do not re-add)

- Server: `PATCH /decks/{id}/rename` (teacher/admin + collaborator gated) accepts
  `UpdateDeck.parent_id` to re-parent a deck, with server-side cycle detection
  ("cannot move a deck under its own descendant" → 400).
- Core (`client/shared/src/app.rs`):
  - `Event::DeckMove { deck_id: i64, parent_id: Option<i64> }` → emits
    `PATCH /decks/{deck_id}/rename` touching **only** `parent_id`.
    - `parent_id: None` → moves the deck to the root (`parent_id: null`).
    - `parent_id: Some(id)` → moves it under that deck.
  - `Event::DeckMoved` completion → on success re-fetches the deck list
    (`DecksRequested`); on error sets `view.decks_error`.
- `view.decks` is already a nested tree (parent/children), so you can present the
  move target as the flattened list of other decks (excluding the deck itself and
  its descendants to avoid cycles, though the server also guards this).
- `view.permissions.can_manage_decks: boolean` — gate the move affordance on this.

## What to build (shell, `client/shells/react/app/components/deck-tree.tsx`)

1. A per-deck **"Move"** affordance (in the row-options dropdown, and/or
   drag-and-drop to re-parent) rendered only when `view.permissions.can_manage_decks`
   is true.

2. A **move dialog** listing valid parent targets:
   - "Top / root" (sends `parent_id: None`),
   - every other deck except the deck being moved **and its descendants** (to
     avoid cycles; the server also rejects them, but filtering client-side avoids
     a pointless 400).

3. On confirm, fire `Event::DeckMove { deck_id, parent_id }`. The core re-fetches
   the list on success; surface `view.decks_error` if it fails (e.g. cycle or not a
   collaborator).

## Notes / decisions

- Do not mutate the tree locally — wait for `DeckMoved` → `DecksRequested`.
- If you also implement drag-and-drop, derive the target `parent_id` from the drop
  target deck's `id` (or `None` for a root drop).
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated
  `shared`; the `DeckMove`/`DeckMoved` events are in the generated bindings).
- Manual: as a teacher/admin, move a deck under another deck and to the root, and
  confirm the tree updates; confirm moving a deck under its own descendant is
  prevented (or surfaces the 400 cleanly). As a student, confirm the affordance is
  absent.
- Commit with Conventional Commits scope `shell/react`.
