# Carve-out (core agent → shell agent): create deck from the deck list

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

Users need to create decks from the deck-list screen. The server endpoint and the
core surface are already done — this is shell UI work only.

## What already exists (consume this — do not re-add)

- Server: `POST /decks` (teacher/admin-gated), returns `DeckResponse` (201). Done.
- Core (`client/shared/src/app.rs`):
  - `Event::DeckCreate { title: String, description: Option<String>, parent_id: Option<i64> }`
    → emits `POST /decks` with a `CreateDeck` body (daily-limit fields default to
    `preset` / `None`).
  - `Event::DeckCreated` completion → on success re-fetches the deck list
    (`DecksRequested`), so the new deck (and its nesting) appears; on error sets
    `view.decks_error`.
- `view.permissions.can_manage_decks: boolean` — gate the create affordance on
  this (students cannot create decks; only teachers/admins).

## What to build (shell, `client/shells/react/app/components/deck-tree.tsx`)

1. A **"New deck"** button / affordance on the deck-list screen, rendered only when
   `view.permissions.can_manage_decks` is true.

2. A **create-deck dialog** with:
   - `title` (required),
   - `description` (optional),
   - `parent` (optional, from the flattened deck tree — "no parent" = root).

3. On submit, fire `Event::DeckCreate { title, description, parent_id }`. The core
   re-fetches the list on success, so the new deck appears automatically; surface
   `view.decks_error` if the create failed (e.g. a bad parent id).

## Notes / decisions

- Nested decks: pass `parent_id: Some(selectedDeckId)` when the user picks a parent,
  otherwise `None` to create a root deck. Do not invent any parent-chain logic — the
  server validates the parent is in the same school.
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated `shared`;
  the new `DeckCreate`/`DeckCreated` events are already in the generated bindings).
- Manual: as a teacher/admin, create a root deck and a nested deck, confirm both
  appear in the list (and the nested one under its parent). As a student, confirm
  the button is absent.
- Commit with Conventional Commits scope `shell/react`.
