-- Add a persisted `interday` flag to the reviews table for Anki-faithful
-- three-bucket daily accounting (new / interday-learning / review).

-- This captures, at review time, whether the answered card was in an
-- interday learning step (step >= 1 day), so later count queries don't have
-- to reconstruct the classification from `deck_option_steps` (which may have
-- changed since the review). See ROADMAP Notes "Three-bucket counts".
ALTER TABLE reviews
    ADD COLUMN interday BOOLEAN NOT NULL DEFAULT FALSE;
