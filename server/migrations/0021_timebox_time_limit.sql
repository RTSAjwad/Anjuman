-- Timebox time limit (US-3.2).
--
-- Persists the user's timebox interval ("show me a summary every N minutes"),
-- 0 = disabled. This is persistence-only: the value has no server-side effect —
-- the timebox popup is a client-side concern (stage 4). Stored in minutes to
-- match Anki's in-app input (0–9999, default 0).

ALTER TABLE user_preferences
    ADD COLUMN timebox_time_limit BIGINT NOT NULL DEFAULT 0;
