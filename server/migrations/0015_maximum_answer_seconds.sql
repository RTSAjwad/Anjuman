-- Maximum answer seconds (US-2.15).
--
-- Caps the recorded response time for a single review. The client measures the
-- elapsed time and sends `response_time_ms`; the server clamps it here. Anki's
-- in-app bound is min 1, max 7200, default 60; the range is enforced in the
-- handlers (a cap of 0 is meaningless), not in a CHECK constraint.

ALTER TABLE deck_options
    ADD COLUMN maximum_answer_seconds BIGINT NOT NULL DEFAULT 60;
