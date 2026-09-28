-- Interday learning/review order (US-2.11).

CREATE TYPE interday_order AS ENUM ('mix', 'before', 'after');

ALTER TABLE deck_options
    ADD COLUMN interday_order interday_order NOT NULL DEFAULT 'mix';
