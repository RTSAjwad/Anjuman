-- Reintroduce the 'mix' value into the new/review and interday order enums
-- (reversing the descope in 0011). The mix is implemented as a stateless
-- Anki `Intersperser`, so it no longer requires a materialized study queue.

-- Recreate both enums with their original three values, then restore the
-- Anki-faithful default of 'mix'.
ALTER TYPE new_review_order RENAME TO new_review_order_before_only;
CREATE TYPE new_review_order AS ENUM ('mix', 'before', 'after');
ALTER TABLE deck_options ALTER COLUMN new_review_order DROP DEFAULT;
ALTER TABLE deck_options ALTER COLUMN new_review_order TYPE new_review_order USING new_review_order::text::new_review_order;
ALTER TABLE deck_options ALTER COLUMN new_review_order SET DEFAULT 'mix';
DROP TYPE new_review_order_before_only;

ALTER TYPE interday_order RENAME TO interday_order_before_only;
CREATE TYPE interday_order AS ENUM ('mix', 'before', 'after');
ALTER TABLE deck_options ALTER COLUMN interday_order DROP DEFAULT;
ALTER TABLE deck_options ALTER COLUMN interday_order TYPE interday_order USING interday_order::text::interday_order;
ALTER TABLE deck_options ALTER COLUMN interday_order SET DEFAULT 'mix';
DROP TYPE interday_order_before_only;

-- Revert 0011's rewrite of the descope placeholder: any preset that was
-- switched from 'mix' to 'after' by 0011 goes back to Anki's 'mix' default.
UPDATE deck_options SET new_review_order = 'mix' WHERE new_review_order = 'after';
UPDATE deck_options SET interday_order = 'mix' WHERE interday_order = 'after';
