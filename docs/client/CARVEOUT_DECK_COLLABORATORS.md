# Carve-out (core agent → shell agent): view & manage deck owners/maintainers

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

Users need to view a deck's owner and maintainers (collaborators), and manage
them (share/unshare/transfer ownership) from the deck-list screen. The server
endpoints and the core surface are already done — this is shell UI work only.

## What already exists (consume this — do not re-add)

- Server (all already exist, teacher/admin + owner-gated):
  - `GET /decks/{id}` → `DeckDetailResponse` (owner + collaborators + classes).
  - `POST /decks/{id}/share` (`{ user_id }`) → add collaborator.
  - `DELETE /decks/{id}/share/{user_id}` → remove collaborator.
  - `PATCH /decks/{id}/owner` (`{ user_id }`) → transfer ownership.
  - `GET /users/search?q=` → `Vec<SearchResult>` (teacher/admin picker; admins
    excluded server-side).
- Core (`client/shared/src/app.rs`) — events:
  - `DeckDetailRequested { deck_id }` → `DeckDetailResult`
  - `DeckShare { deck_id, user_id }` → `DeckShared`
  - `DeckUnshare { deck_id, user_id }` → `DeckUnshared`
  - `DeckTransferOwner { deck_id, user_id }` → `DeckOwnerTransferred`
  - `UsersSearchRequested { q }` → `UsersSearchResult`
- `ViewModel` fields to render from:
  - `deck_detail: Option<DeckDetailView>` — `{ owner_user_id, owner_email,
    owner_first_name, owner_last_name, collaborators: [{user_id, email,
    first_name, last_name, shared_at}], classes: [{id, name}] }`
  - `deck_detail_error: Option<String>`
  - `user_search: Vec<UserSearchView>` — `{ id, email, first_name, last_name }`
  - `user_search_error: Option<String>`
- `view.permissions.can_manage_decks: boolean` — gate the *management* actions
  (share/unshare/transfer) on this. Viewing is available to any user who can see
  the deck (but list_deck_classes/get_deck already hide admin meta from
  students; non-owner collaborators can view but not manage).

## What to build (shell, `client/shells/react/app/components/deck-tree.tsx` + a dialog)

1. A per-deck **"Manage" / "Collaborators"** row action that opens a dialog, firing
   `DeckDetailRequested { deck_id }` on open (to populate `view.deck_detail`).

2. The dialog renders:
   - **Owner** — name + email from `deck_detail.owner_*`.
   - **Collaborators** — list of `deck_detail.collaborators` (name/email), each
     with an "Unshare" action (only when `can_manage_decks`).
   - **Classes** — list of `deck_detail.classes` (read-only for now; class
     management is a separate feature).

3. **Manage actions** (gated on `can_manage_decks`):
   - **Share** — a search box driving `UsersSearchRequested { q }` → render
     `view.user_search`; selecting a user fires `DeckShare { deck_id, user_id }`.
   - **Unshare** — per-collaborator "Unshare" fires `DeckUnshare { deck_id, user_id }`.
   - **Transfer ownership** — pick a user and fire `DeckTransferOwner { deck_id, user_id }`.

4. After each action the core re-fetches the detail (and the deck list on
   transfer), so the dialog re-renders automatically. Surface `deck_detail_error`
   and `user_search_error` on failure.

## Notes / decisions

- Viewing vs managing: the owner (and admin) can manage; collaborators/students
  can view but not share/unshare/transfer. Gate mutating affordances on
  `can_manage_decks`; the server also enforces owner-only on the write endpoints.
- The share/transfer picker is `GET /users/search` — debounce the input and don't
  search on every keystroke.
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.
- Class management (assign/remove deck ↔ class) is intentionally out of scope for
  this dialog; `classes` is surfaced read-only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated
  `shared`; the new events are in the generated bindings).
- Manual: as owner, open the dialog and confirm owner + collaborators + classes
  render; share with a teacher, unshare, and transfer ownership, confirming the
  dialog/deck list update each time. As a collaborator or student, confirm the
  manage actions are gated.
- Commit with Conventional Commits scope `shell/react`.
