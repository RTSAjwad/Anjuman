-- FSRS parameters (US-2.18a).
--
-- Stores a per-preset FSRS weight vector as JSONB. Empty/`[]` means "use the
-- FSRS default parameters" (`FSRS::new(&[]`)). `apply_review` consumes these
-- weights instead of always using `FSRS::default()`. The optimizer that would
-- produce/store optimized weights is a separate deferred story (US-2.18b).

ALTER TABLE deck_options
    ADD COLUMN fsrs_parameters JSONB NOT NULL DEFAULT '[]';
