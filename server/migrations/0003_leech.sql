-- Leech tracking (deck options) — FSRS-only, no SM-2.
--
-- Adds the leech threshold (number of review-card "Again" presses before a card
-- is marked a leech) and the leech action (tag-only vs suspend-card). Also adds
-- a minimal `leech_tagged_at` marker on notes so "is a leech" is queryable
-- without a full tag system (see ROADMAP stage 6).

--------------------------------------------------------------------
-- Leech action enum
--------------------------------------------------------------------

CREATE TYPE leech_action AS ENUM ('tag_only', 'suspend_card');

--------------------------------------------------------------------
-- deck_options: leech threshold + action
--------------------------------------------------------------------

ALTER TABLE deck_options
    ADD COLUMN leech_threshold BIGINT NOT NULL DEFAULT 8;

ALTER TABLE deck_options
    ADD COLUMN leech_action leech_action NOT NULL DEFAULT 'suspend_card';

--------------------------------------------------------------------
-- notes: minimal leech marker (no general tag system yet)
--------------------------------------------------------------------

ALTER TABLE notes
    ADD COLUMN leech_tagged_at TIMESTAMPTZ;
