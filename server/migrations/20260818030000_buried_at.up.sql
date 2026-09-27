-- Rework burial from an absolute "unbury at" timestamp to a "buried at"
-- timestamp, matching Anki's query-time day-rollover semantics.
--
-- Old model: `buried_until` stored the absolute epoch at which the card would
--   automatically unbury. The "is it buried?" check compared against now.
--
-- New model: `buried_at` stores when the card was buried. A card is buried iff
--   `buried_at >= <start of today>`, where the day boundary is the user's
--   day-start hour. This makes unbury automatic on day rollover with no
--   background job, and lets a day-start change take effect immediately.

-- Best-effort backfill: a card that was still buried (buried_until in the
-- future) was buried ~one day ago under the old "next UTC midnight" rule, so
-- we approximate buried_at = buried_until - 86400. Cards already unburied stay
-- NULL. This only affects transiently-buried cards and is intentionally lossy.
ALTER TABLE student_card_states ADD COLUMN buried_at INTEGER;

UPDATE student_card_states
SET buried_at = CASE
    WHEN buried_until IS NOT NULL AND buried_until > unixepoch()
        THEN buried_until - 86400
    ELSE NULL
END;

ALTER TABLE student_card_states DROP COLUMN buried_until;
