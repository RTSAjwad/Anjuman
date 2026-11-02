# Carve-out (core agent → shell agent): update deck details from the deck list

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

Users need to edit a deck's details (rename + description) from the deck-list
screen. The server endpoint and the core surface are already done — this is shell
UI work only.

## What already exists (consume this — do not re-add)

- Server: `PATCH /decks/{id}/rename` (teacher/admin + collaborator gated) — a
  general "update deck details" endpoint accepting `UpdateDeck` (`title`,
  `description`, `parent_id`, and per-deck daily-limit fields). We use it for
  title + description here.
- Core (`client/shared/src/app.rs`):
  - `Event::DeckUpdate { deck_id: i64, title: String, description: Option<String> }`
    → emits `PATCH /decks/{deck_id}/rename`.
    - `title` → always sent (rename).
    - `description` → `Some(text)` sets it; `Some("")` **or** `None` clears it to
      `null` (the server stores `NULL`).
    - `parent_id` and daily-limit fields are left unchanged (out of scope here).
  - `Event::DeckUpdated` completion → on success re-fetches the deck list
    (`DecksRequested`); on error sets `view.decks_error`.
- `view.decks` already exposes `title`, `description`, and `parent_id` per deck, so
  you can prefill an edit dialog.
- `view.permissions.can_manage_decks: boolean` — gate the edit affordance on this
  (students cannot edit deck details).

## What to build (shell, `client/shells/react/app/components/deck-tree.tsx`)

1. A per-deck **row action** (e.g. an "Edit" / rename entry in the existing
   row-options dropdown) rendered only when `view.permissions.can_manage_decks`
   is true.

2. An **edit-deck dialog** prefilled from the selected deck's `title` and
   `description`, with:
   - `title` (required; reject empty before submit),
   - `description` (optional; leaving it empty = clear the description).

3. On submit, fire `Event::DeckUpdate { deck_id, title, description }`. The core
   re-fetches the list on success; surface `view.decks_error` if it fails (e.g.
   not a collaborator, or empty title).

## Notes / decisions

- You do **not** need to send `parent_id` — parent-move is a separate, future
  feature (drag-to-reparent) and is intentionally out of scope here.
- `description` semantics: send `Some(text)` to set, `Some("")` to clear, or omit
  to leave unchanged. Since the dialog is always prefilled, you'll normally send
  the current-or-edited value; an empty description means "clear".
- Do not mutate the tree locally — wait for `DeckUpdated` → `DecksRequested`.
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated
  `shared`; the `DeckUpdate`/`DeckUpdated` events are in the generated bindings).
- Manual: as a teacher/admin, rename a deck and edit/clear its description and
  confirm the list reflects it; as a student, confirm the affordance is absent.
- Commit with Conventional Commits scope `shell/react`.
