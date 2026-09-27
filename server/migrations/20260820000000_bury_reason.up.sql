-- Track *why* a card is buried, so sibling-bury and user-bury can be
-- distinguished. `buried_at` remains the "am I buried + when" signal;
-- `bury_reason` is a label stored alongside it (both NULL when not buried).
--
-- Values:
--   user_card, user_note          — explicit user bury (card / note)
--   sibling_new, sibling_review,
--   sibling_interday              — automatically buried because a sibling card
--                                   (same note) was answered earlier

ALTER TABLE student_card_states ADD COLUMN bury_reason TEXT;
