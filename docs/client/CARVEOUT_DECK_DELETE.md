# Carve-out (core agent → shell agent): delete deck from the deck list

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

Users need to delete decks from the deck-list screen. The server endpoint and the
core surface are already done — this is shell UI work only.

## What already exists (consume this — do not re-add)

- Server: `DELETE /decks/{id}` (teacher/admin + deck-owner gated), returns
  `MessageResponse`. Two modes via the `cascade` query param:
  - `cascade` omitted/false → delete the deck only; children are re-parented to
    the root (`parent_id = NULL`).
  - `cascade=true` → delete the deck and all descendants recursively.
- Core (`client/shared/src/app.rs`):
  - `Event::DeckDelete { deck_id: i64, cascade: bool }` → emits
    `DELETE /decks/{deck_id}` (with `?cascade=true` only when `cascade` is true).
  - `Event::DeckDeleted` completion → on success re-fetches the deck list
    (`DecksRequested`); on error sets `view.decks_error`.
- `view.permissions.can_manage_decks: boolean` — gate the delete affordance on
  this (students cannot delete decks).

## What to build (shell, `client/shells/react/app/components/deck-tree.tsx`)

1. A per-deck **row action** (e.g. in the existing row-options dropdown, or a
   delete button/icon) rendered only when `view.permissions.can_manage_decks` is
   true.

2. A **delete confirmation** flow:
   - If the deck has children (`children.length > 0`), ask the user to choose
     between **"Delete deck only"** (children move to the root) and **"Delete deck
     and subdecks"**.
   - If the deck has no children, a single confirm suffices.

3. On confirm, fire `Event::DeckDelete { deck_id, cascade }` where `cascade` is
   `true` only for the "and subdecks" choice. The core re-fetches the list on
   success; surface `view.decks_error` if it fails (e.g. not the deck owner).

## Notes / decisions

- Do not delete locally — wait for the core's `DeckDeleted` → `DecksRequested`
  round-trip so the tree reflects the authoritative server state.
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated
  `shared`; the `DeckDelete`/`DeckDeleted` events are in the generated bindings).
- Manual: as a teacher/admin, delete a leaf deck (single confirm) and a parent deck
  (choose both modes) and confirm the list updates correctly. As a student, confirm
  the affordance is absent.
- Commit with Conventional Commits scope `shell/react`.
