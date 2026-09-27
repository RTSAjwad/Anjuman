-- Reverse the bury model change: re-add `buried_until` as an absolute
-- "unbury at" timestamp, approximating from `buried_at`. Lossy by design.
ALTER TABLE student_card_states ADD COLUMN buried_until INTEGER;

UPDATE student_card_states
SET buried_until = CASE
    WHEN buried_at IS NOT NULL THEN buried_at + 86400
    ELSE NULL
END;

ALTER TABLE student_card_states DROP COLUMN buried_at;
