-- New/review order (US-2.10).

CREATE TYPE new_review_order AS ENUM ('mix', 'before', 'after');

ALTER TABLE deck_options
    ADD COLUMN new_review_order new_review_order NOT NULL DEFAULT 'mix';
