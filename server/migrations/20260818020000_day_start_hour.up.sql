-- Day-start hour (Anki's "Next day starts at", default 4 AM).
--
-- A per-user scheduling option used to bucket "today" for daily limits,
-- burial auto-expiry, and (later) streaks. Stored as an integer hour 0-23.
-- The day boundary is anchored to UTC for now (no per-user timezone yet).

ALTER TABLE user_preferences ADD COLUMN day_start_hour INTEGER NOT NULL DEFAULT 4;
