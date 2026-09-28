-- New card sort order (US-2.9).

CREATE TYPE new_sort_order AS ENUM ('card_type_then_gathered', 'gathered', 'card_type_then_random', 'random_note_then_card_type', 'random');

ALTER TABLE deck_options
    ADD COLUMN new_sort_order new_sort_order NOT NULL DEFAULT 'card_type_then_gathered';
