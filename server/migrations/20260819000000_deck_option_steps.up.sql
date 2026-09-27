-- Normalise deck-option scheduling steps into their own table so step data
-- can be joined in SQL (e.g. for intraday/interday learning classification).
--
-- This also introduces a "system" school (id = 0) that owns a single global
-- default preset. Decks with `options_id = NULL` fall back to this preset,
-- replacing the previous in-memory default with a real, joinable row. Real
-- schools have id >= 1 and therefore can never own or mutate the global
-- default.

-- 1. Seed the system school (owns the global default preset).
INSERT OR IGNORE INTO schools (id, name, created_at)
VALUES (0, 'System', '1700000000');

-- 2. Create the normalised steps table.
CREATE TABLE deck_option_steps (
    id INTEGER PRIMARY KEY,
    options_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('learning', 'relearning')),
    step_index INTEGER NOT NULL,
    seconds INTEGER NOT NULL,
    FOREIGN KEY (options_id) REFERENCES deck_options(id) ON DELETE CASCADE,
    UNIQUE (options_id, kind, step_index)
);

CREATE INDEX idx_deck_option_steps_options
ON deck_option_steps(options_id, kind, step_index);

-- 3. Seed the global default preset under the system school. Its steps mirror
--    the previous in-memory default_options() (60s, 600s learning; 600s
--    relearning).
INSERT OR IGNORE INTO deck_options (id, school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, created_at)
VALUES (0, 0, 'Default', 0.9, 0, 0, 0, 20, 200, 1700000000);

INSERT OR IGNORE INTO deck_option_steps (options_id, kind, step_index, seconds)
VALUES
    (0, 'learning',   0, 60),
    (0, 'learning',   1, 600),
    (0, 'relearning', 0, 600);

-- 4. Drop the now-obsolete comma-separated step columns. (In the migration
--    applied against a fresh dev database there are no other presets to
--    backfill; any production data would be migrated row-by-row first.)
ALTER TABLE deck_options DROP COLUMN learning_steps;
ALTER TABLE deck_options DROP COLUMN relearning_steps;
