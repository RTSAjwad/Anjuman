-- Seed nested subdecks (and their cards) for dev/test (idempotent).
--
-- Adds two subdecks under the seed "Biology 101" deck (id 1) so the client's
-- nested deck-tree rendering has data to exercise, plus three cards in each.
-- Uses `ON CONFLICT DO NOTHING` so re-running is safe.

--------------------------------------------------------------------
-- Nested subdecks under Biology 101 (id 1)
--------------------------------------------------------------------

INSERT INTO decks (id, school_id, title, description, created_by, parent_id, created_at)
VALUES
    (2, 1, 'Cell Biology', 'Subdeck: cell structure and organelles', 1, 1, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second'),
    (3, 1, 'Genetics',    'Subdeck: DNA and heredity', 1, 1, 'epoch'::TIMESTAMPTZ + 1700000001 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

--------------------------------------------------------------------
-- Notes for the subdecks
--------------------------------------------------------------------

INSERT INTO notes (id, note_type_id, fields_json, created_at)
VALUES
    -- Subdeck: Cell Biology (deck 2)
    (11, 1, '{"Front":"In which organelle does cellular respiration occur?","Back":"Mitochondrion"}', 'epoch'::TIMESTAMPTZ + 1700000110 * INTERVAL '1 second'),
    (12, 1, '{"Front":"What rigid structure surrounds plant cells?","Back":"Cell wall"}', 'epoch'::TIMESTAMPTZ + 1700000111 * INTERVAL '1 second'),
    (13, 1, '{"Front":"Which organelle is the site of photosynthesis?","Back":"Chloroplast"}', 'epoch'::TIMESTAMPTZ + 1700000112 * INTERVAL '1 second'),
    -- Subdeck: Genetics (deck 3)
    (14, 1, '{"Front":"What are the monomers of DNA?","Back":"Nucleotides"}', 'epoch'::TIMESTAMPTZ + 1700000113 * INTERVAL '1 second'),
    (15, 1, '{"Front":"How many chromosomes do humans typically have?","Back":"46"}', 'epoch'::TIMESTAMPTZ + 1700000114 * INTERVAL '1 second'),
    (16, 1, '{"Front":"What is a segment of DNA that codes for a protein?","Back":"A gene"}', 'epoch'::TIMESTAMPTZ + 1700000115 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

--------------------------------------------------------------------
-- Cards for the subdecks (Cell Biology = deck 2, Genetics = deck 3)
--------------------------------------------------------------------

INSERT INTO cards (id, note_id, deck_id, template_id, created_at)
SELECT v.id, v.note_id, v.deck_id, t.id, v.created_at
FROM (
    VALUES
        (11, 11, 2, 'epoch'::TIMESTAMPTZ + 1700000110 * INTERVAL '1 second'),
        (12, 12, 2, 'epoch'::TIMESTAMPTZ + 1700000111 * INTERVAL '1 second'),
        (13, 13, 2, 'epoch'::TIMESTAMPTZ + 1700000112 * INTERVAL '1 second'),
        (14, 14, 3, 'epoch'::TIMESTAMPTZ + 1700000113 * INTERVAL '1 second'),
        (15, 15, 3, 'epoch'::TIMESTAMPTZ + 1700000114 * INTERVAL '1 second'),
        (16, 16, 3, 'epoch'::TIMESTAMPTZ + 1700000115 * INTERVAL '1 second')
) AS v(id, note_id, deck_id, created_at)
CROSS JOIN LATERAL (
    SELECT id FROM note_type_templates WHERE note_type_id = 1 AND ord = 0 LIMIT 1
) AS t
ON CONFLICT (id) DO NOTHING;
