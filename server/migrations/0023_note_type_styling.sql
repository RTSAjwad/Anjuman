-- Note-type styling (US-4.6).
--
-- Adds the shared "Styling" CSS block to a note type, matching Anki's per-note
-- type styling (one block shared by every card/template of that type). Cards are
-- rendered from the template + this styling, so each card is self-contained and
-- carries its author-defined appearance.
--
-- Also seeds Anki's default `.card` styling onto the existing seed note types so
-- untouched note types render sanely.

ALTER TABLE note_types
    ADD COLUMN styling TEXT NOT NULL DEFAULT '';

-- Backfill the existing seed note types with Anki's default `.card` styling.
UPDATE note_types
SET styling = '.card {
    font-family: arial;
    font-size: 20px;
    text-align: center;
    color: black;
    background-color: white;
}'
WHERE id IN (1, 2);
