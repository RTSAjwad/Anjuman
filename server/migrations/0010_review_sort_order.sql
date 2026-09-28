-- Review sort order (US-2.12).

CREATE TYPE review_sort_order AS ENUM (
    'due_then_random',
    'due_then_deck',
    'deck_then_due',
    'ascending_interval',
    'descending_interval',
    'easy_first',
    'difficult_first',
    'ascending_retrievability',
    'descending_retrievability',
    'relative_overdueness',
    'random',
    'order_added',
    'latest_added_first'
);

ALTER TABLE deck_options
    ADD COLUMN review_sort_order review_sort_order NOT NULL DEFAULT 'due_then_random';
