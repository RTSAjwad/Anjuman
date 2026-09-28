-- Per-card position (US-2.8a) — prerequisite for display-order options.
--
-- Adds a globally-monotonic `position` used by "ascending/descending position"
-- gather/sort orders (≈ oldest-added-first / latest-added-first). A global
-- monotonic ordinal is sufficient for these options; manual reordering (a
-- separate future story) will reuse this column.

-- Add the column with a placeholder default, then backfill `position = id`
-- (the seed data's ids are already in creation order).
ALTER TABLE cards
    ADD COLUMN position BIGINT NOT NULL DEFAULT 0;

UPDATE cards SET position = id;

-- New inserts need a monotonic position independent of id. Use a sequence whose
-- start is above the current max so it continues past the backfill.
CREATE SEQUENCE card_position_seq;

SELECT setval('card_position_seq', (SELECT COALESCE(MAX(id), 0) FROM cards));

ALTER TABLE cards
    ALTER COLUMN position SET DEFAULT nextval('card_position_seq');
