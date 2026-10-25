# Carve-out (shell agent → core agent): `notes_id_seq` out of sync with seeded notes → POST /notes 500s (db scope)

**From:** shell agent
**To:** core agent
**Status:** pending (blocking note creation)

## Symptom

Creating a note (`POST /notes`, wired from the shell's "Add note" dialog) returns
**HTTP 500 "Database error"**. Reproduced directly against the API:

```
POST /notes  {"note_type_id":1,"deck_id":1,"fields":{"Front":"f","Back":"b"}}
→ 500 {"message":"Database error"}   (error body masked by the handler)
```

## Root cause

The `notes` table's identity sequence (`notes_id_seq`) is **out of sync** with its
rows. The seed migrations insert notes with **explicit ids** but never advance the
sequence:

- `server/migrations/0002_seed_data.sql:99` — `INSERT INTO notes (id, …)`
- `server/migrations/0022_seed_nested_decks.sql:21` — `INSERT INTO notes (id, …)`

There is **no `setval('notes_id_seq', …)`** anywhere (the only `setval` is for the
unrelated `card_position_seq` in `0005_cards_position.sql`).

Consequence (verified live against `anjuman` DB):

```
SELECT id FROM notes ORDER BY id;      -- 1..16 (16 rows)
SELECT last_value, is_called FROM notes_id_seq;  -- last_value=5
```

So `INSERT INTO notes (note_type_id, fields_json, created_at) … RETURNING id`
gets `id=6` from the sequence, which collides with the existing note id=6 →
`duplicate key value violates unique constraint "notes_pkey"` → the handler maps
it to the generic 500.

## What to do (db scope)

Advance the identity sequence past the seeded ids. Options (pick one):

1. **Migration-level fix (preferred, permanent):** add a migration that runs
   `SELECT setval('notes_id_seq', (SELECT COALESCE(MAX(id), 0) FROM notes));`
   (mirrors how `0005_cards_position.sql` resets `card_position_seq`). This makes
   fresh DBs correct without manual intervention.

2. **Live fix** (immediate unblock for already-provisioned dev DBs): run
   `SELECT setval('notes_id_seq', (SELECT MAX(id) FROM notes));`

Note: check the **other identity-sequence-backed tables that seed with explicit
ids** too (`schools`, `users`, `decks`, `note_types`, `note_type_templates`, etc.
in `0002_seed_data.sql`) — the same `setval` drift may affect them; at minimum
`notes` blocks note creation.

## Not my scope

`server/migrations/` + the live DB are core-agent/db scope. I'm only reporting;
the shell's `NoteCreate` event + dialog are correct and already fire the right
request.

## Validation

- After the fix, `POST /notes` (teacher) returns `201` with the new note (and its
  generated cards), and a second create succeeds with a fresh id.
- `server/tests/` note-creation tests (if any assert ids) still pass.

## Not changing

No `contracts/`/`client/shared/` change.
