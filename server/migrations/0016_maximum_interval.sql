-- Maximum review interval (US-2.17).
--
-- Caps the FSRS review interval (in days) so Hard/Good/Easy converge at the cap.
-- Applies to the review interval only — not learning/relearning steps, and not
-- the stored FSRS stability. Anki's in-app bound is min 0, max 36500 (default
-- 36500 ≈ 100 years); the scheduler floors the maximum at 1 (`maximum.max(1)`),
-- so 0 is redundant with 1 and we validate `1..=36500` in the handlers instead.

ALTER TABLE deck_options
    ADD COLUMN maximum_interval BIGINT NOT NULL DEFAULT 36500;
