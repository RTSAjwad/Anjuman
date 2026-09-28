-- New card gather order (US-2.8).

CREATE TYPE new_gather_order AS ENUM ('deck', 'ascending', 'descending', 'random_notes', 'random_cards');

ALTER TABLE deck_options
    ADD COLUMN new_gather_order new_gather_order NOT NULL DEFAULT 'deck';
