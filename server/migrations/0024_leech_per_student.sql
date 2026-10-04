-- US-2.20 — leech as per-student state (not a note-level tag).
--
-- Replace the shared-note `notes.leech_tagged_at` marker with a per-(student,
-- card) `student_card_states.leech_at`. A leech is a *personal* scheduling
-- signal (one student forgetting a card must not mark it for others), so it
-- lives on the already-per-student card state, not the shared note.

ALTER TABLE student_card_states
    ADD COLUMN leech_at TIMESTAMPTZ;

-- The old note-level marker is superseded and now leaks across students.
ALTER TABLE notes
    DROP COLUMN leech_tagged_at;
