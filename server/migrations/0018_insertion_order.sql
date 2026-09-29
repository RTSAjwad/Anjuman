-- New-card insertion order (US-2.13).
--
-- Controls the `cards.position` (due #) assigned to new cards. `sequential`
-- (default) appends monotonically; `random` assigns a random position on insert
-- and re-sorts existing new cards when the setting changes (scoped to the
-- preset's decks — a documented divergence from Anki's global position space).

CREATE TYPE insertion_order AS ENUM ('sequential', 'random');

ALTER TABLE deck_options
    ADD COLUMN insertion_order insertion_order NOT NULL DEFAULT 'sequential';
