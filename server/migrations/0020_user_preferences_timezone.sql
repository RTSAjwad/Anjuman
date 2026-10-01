-- Per-user timezone (US-3.1).
--
-- "Next day starts at" (`day_start_hour`) is anchored to UTC today, which is
-- wrong for any student not in UTC. Store a per-user IANA timezone name so the
-- study-day boundary is computed against the user's local wall-clock time.
--
-- Default `'UTC'` preserves current behaviour exactly until a client/onboarding
-- sets a real zone (see `PREFERENCES_SUPPORT.md`). Stored as a name rather than
-- a fixed `UTC+hh` offset because an offset breaks across DST transitions.

ALTER TABLE user_preferences
    ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC';
