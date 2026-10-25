-- Advance identity sequences past the explicitly-id'ed seed rows.
--
-- The seed migrations (0002_seed_data.sql, 0022_seed_nested_decks.sql) INSERT
-- rows with explicit `id`s into identity-backed tables, but never advance their
-- identity sequences. The sequences therefore sit at their initial value while
-- the tables already contain higher ids, so the next `INSERT ... RETURNING id`
-- collides with an existing row (`duplicate key value violates unique constraint`)
-- and surfaces as a generic 500 on the API (first seen on `POST /notes`, which
-- blocked note creation).
--
-- Fix: advance each affected sequence to the table's current MAX(id), mirroring
-- how 0005_cards_position.sql resets `card_position_seq`.
--
-- `GREATEST(COALESCE(MAX(id), 0), 1)` keeps the statement valid when a table is
-- empty (or only holds the id-0 system/default row), because `BIGINT ... AS
-- IDENTITY` sequences have MINVALUE 1 and a `setval(..., 0)` would be out of
-- bounds. Idempotent: re-running re-sets to the same MAX(id).

SELECT setval(pg_get_serial_sequence('schools', 'id'),  (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM schools));
SELECT setval(pg_get_serial_sequence('users', 'id'),    (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM users));
SELECT setval(pg_get_serial_sequence('classes', 'id'),  (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM classes));
SELECT setval(pg_get_serial_sequence('deck_options', 'id'), (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM deck_options));
SELECT setval(pg_get_serial_sequence('note_types', 'id'), (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM note_types));
SELECT setval(pg_get_serial_sequence('decks', 'id'),    (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM decks));
SELECT setval(pg_get_serial_sequence('notes', 'id'),    (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM notes));
SELECT setval(pg_get_serial_sequence('cards', 'id'),    (SELECT GREATEST(COALESCE(MAX(id), 0), 1) FROM cards));
