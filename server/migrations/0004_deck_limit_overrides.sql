-- Per-deck daily-limit overrides (US-2.6).
--
-- Adds the Anki `preset` / `this deck` / `today only` three-way selector for the
-- new-cards/day and max-reviews/day limits, stored per-deck (school-authored).
-- The preset-scoped limit remains on `deck_options`; these columns override it
-- for a single deck without forking the shared preset.

CREATE TYPE limit_mode AS ENUM ('preset', 'this_deck', 'today_only');

ALTER TABLE decks
    ADD COLUMN new_per_day_mode limit_mode NOT NULL DEFAULT 'preset',
    ADD COLUMN review_per_day_mode limit_mode NOT NULL DEFAULT 'preset',
    ADD COLUMN new_per_day_override BIGINT,
    ADD COLUMN review_per_day_override BIGINT,
    -- Date the `today_only` override was set, for lazy expiry at the next
    -- study-day boundary (computed from the student's `day_start_hour`).
    ADD COLUMN new_per_day_today_date DATE,
    ADD COLUMN review_per_day_today_date DATE;
