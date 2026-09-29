-- Easy Days — per-weekday workload (US-2.16, persist-only).
--
-- Selection-only: the server stores the 7-value selection (Monday-first) but
-- performs no scheduling effect — Anki's Easy Days is wired into its interval
-- load balancer, which we do not implement (see ROADMAP US-2.16). Stored as
-- JSONB to mirror the `field_names` precedent and avoid enum-array friction;
-- each element is one of "minimum" / "reduced" / "normal".

ALTER TABLE deck_options
    ADD COLUMN easy_days JSONB NOT NULL DEFAULT '["normal","normal","normal","normal","normal","normal","normal"]';
