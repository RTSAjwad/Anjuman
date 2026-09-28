-- Drop the 'mix' value from the new/review and interday order enums (descope of
-- US-2.10/2.11's `mix`, which we could not implement faithfully without a
-- materialized study queue). Existing presets using 'mix' are rewritten to the
-- new default 'after' (reviews-first), preserving their observed ordering.

-- 1. Redirect the default away from 'mix' before dropping it.
ALTER TABLE deck_options ALTER COLUMN new_review_order SET DEFAULT 'after';
ALTER TABLE deck_options ALTER COLUMN interday_order SET DEFAULT 'after';

-- 2. Rewrite any preset currently set to 'mix'.
UPDATE deck_options SET new_review_order = 'after' WHERE new_review_order = 'mix';
UPDATE deck_options SET interday_order = 'after' WHERE interday_order = 'mix';

-- 3. Remove the now-unused enum value.
ALTER TYPE new_review_order RENAME TO new_review_order_old;
CREATE TYPE new_review_order AS ENUM ('before', 'after');
ALTER TABLE deck_options ALTER COLUMN new_review_order DROP DEFAULT;
ALTER TABLE deck_options ALTER COLUMN new_review_order TYPE new_review_order USING new_review_order::text::new_review_order;
ALTER TABLE deck_options ALTER COLUMN new_review_order SET DEFAULT 'after';
DROP TYPE new_review_order_old;

ALTER TYPE interday_order RENAME TO interday_order_old;
CREATE TYPE interday_order AS ENUM ('before', 'after');
ALTER TABLE deck_options ALTER COLUMN interday_order DROP DEFAULT;
ALTER TABLE deck_options ALTER COLUMN interday_order TYPE interday_order USING interday_order::text::interday_order;
ALTER TABLE deck_options ALTER COLUMN interday_order SET DEFAULT 'after';
DROP TYPE interday_order_old;
