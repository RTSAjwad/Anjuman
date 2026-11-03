# Carve-out (core agent → shell agent): note-type editor (top-level route)

**From:** core agent
**To:** shell agent
**Status:** pending

## Context

A typed note-type editor as a top-level route (like Cards/Notes), covering CRUD
for note types and their card templates. The server and the core surface are
already done — this is shell UI work only.

## What already exists (consume this — do not re-add)

- Server (all already exist, teacher/admin-gated):
  - `GET /note-types`, `GET /note-types/{id}`, `POST /note-types/{id}/clone`,
    `PATCH /note-types/{id}`, `DELETE /note-types/{id}`,
    `POST /note-types/{id}/templates`,
    `PATCH /note-types/{id}/templates/{template_id}`,
    `DELETE /note-types/{id}/templates/{template_id}`,
    `PATCH /note-types/{id}/templates/order`.
- Core (`client/shared/src/app.rs`) events:
  - `ShowNoteTypes` (top-level nav → `CurrentRoute::NoteTypes`)
  - `NoteTypesRequested` / `NoteTypesResult` → `view.note_types`
  - `NoteTypeSelect { note_type_id }` / `NoteTypeResult` → `view.selected_note_type`
  - `NoteTypeClone { note_type_id }` (the **only** way to make a new note type —
    Anki's "Add note type" clones an existing one)
  - `NoteTypeUpdate { note_type_id, name, field_names, sort_field, styling }` (all `Option`)
  - `NoteTypeDelete { note_type_id }`
  - `NoteTypeTemplateCreate { note_type_id, name, front_pattern, back_pattern }`
  - `NoteTypeTemplateUpdate { note_type_id, template_id, name, front_pattern, back_pattern }` (all `Option`)
  - `NoteTypeTemplateDelete { note_type_id, template_id }`
  - `NoteTypeTemplatesReorder { note_type_id, template_ids }`
- `ViewModel` fields:
  - `note_types: Vec<NoteTypeView>` — `{ id, name, field_names, sort_field,
    styling, templates: [{id, ord, name, front_pattern, back_pattern}], note_count }`
  - `selected_note_type: Option<NoteTypeView>` — the one open in the editor
  - `note_types_error: Option<String>`
  - `permissions.can_manage_note_types: boolean` — gate everything on this
    (students don't see the route).

After each mutation the core re-fetches the list and re-selects the open type, so
the editor re-renders automatically; surface `note_types_error` on failure.

## What to build (shell)

1. A **sidebar NavItem "Note types"** (next to Decks/Cards/Notes), gated on
   `can_manage_note_types`, firing `ShowNoteTypes`. Add a `match` arm for
   `CurrentRoute::NoteTypes` in the root render (mirroring Cards/Notes).

2. **List + editor layout** (two panes):
   - Left: `view.note_types` (name + `note_count`); click → `NoteTypeSelect`.
   - Right: the selected `view.selected_note_type` editor.

3. **Note-type level controls** (all gated on `can_manage_note_types`):
   - **"New note type"** → pick a base note type and fire
     `NoteTypeClone { note_type_id: base.id }` (Anki parity — there is no
     blank-slate create; new types are clones of an existing one).
   - Edit name, field names, sort field (dropdown over `field_names`), and the
     `styling` CSS textarea → `NoteTypeUpdate`.
   - "Delete" (confirm) → `NoteTypeDelete`.

4. **Template level controls** (within the selected type):
   - List the type's `templates` (name, front/back pattern).
   - "Add template" → `NoteTypeTemplateCreate` (name + `front_pattern` +
     `back_pattern`, using `{{Field}}` placeholders).
   - Edit a template → `NoteTypeTemplateUpdate`.
   - Delete a template (confirm) → `NoteTypeTemplateDelete`.
   - Reorder (drag or up/down) → `NoteTypeTemplatesReorder` with the full ordered
     `template_id` list.

## Notes / decisions

- **"New note type" is clone-only** — there is deliberately **no** create-from-
  scratch endpoint/event (Anki parity). The shell's "New" flow is: pick a base
  type → `NoteTypeClone` → the clone is auto-selected; then rename/edit it.
- **Fields are free text**, not a strict schema editor. Collapsing/renaming fields
  with existing notes is a separate (future) concern — this editor edits the note
  type definition itself; the server validates field presence on note create.
- `sort_field` is edited as a dropdown over the current `field_names` (empty =
  "first field", Anki default). `styling` is a raw textarea (the same CSS the
  card injects; see the night-mode carve-out for how it's consumed at study time).
- No shell-local HTTP; reads come from `view()`, writes are `Event`s only.
- No `contracts/`/`server/`/`client/shared/` edits — this is your scope only.

## Validation

- `cargo build` in the client workspace (shell compiles against the updated
  `shared`; the new events are in the generated bindings).
- Manual: as a teacher/admin, open the route, clone a note type, add/edit/reorder/
  delete templates, edit name/fields/sort/styling, and delete a type — confirming
  the list + editor re-render each time. As a student, confirm the route/
  affordances are absent.
- Commit with Conventional Commits scope `shell/react`.
