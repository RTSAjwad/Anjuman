-- Capture the card's scheduling state before a review is applied, so the
-- daily new/review limits can be derived from the reviews table without
-- reconstructing state from history.
--
-- Values mirror student_card_states.state: 'new', 'learning', 'review',
-- 'relearning'. Default to 'review' for any pre-existing rows, which is the
-- safe choice for limit accounting (a review is the conservative bucket).

ALTER TABLE reviews ADD COLUMN state_before TEXT NOT NULL DEFAULT 'review';
