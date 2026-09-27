-- Note-type / template identity refactor.
--
-- Templates get a stable, auto-increment `id` plus an explicit `ord` (display
-- order within the note type). Cards now reference a template by `id` instead
-- of the positional `template_index`, so removing a specific template removes
-- exactly its own cards (identity-based, not position-based).

PRAGMA foreign_keys = OFF;

-- 1. Rebuild note_type_templates with id + ord.
CREATE TABLE note_type_templates_new (
    id INTEGER PRIMARY KEY,
    note_type_id INTEGER NOT NULL,
    ord INTEGER NOT NULL,
    name TEXT NOT NULL,
    front_pattern TEXT NOT NULL,
    back_pattern TEXT NOT NULL,
    FOREIGN KEY (note_type_id) REFERENCES note_types(id) ON DELETE CASCADE,
    UNIQUE (note_type_id, ord)
);

-- Preserve existing templates; `template_index` becomes the initial `ord`.
-- Drop orphan rows whose note type no longer exists (defensive; shouldn't
-- normally occur with foreign_keys on).
INSERT INTO note_type_templates_new (note_type_id, ord, name, front_pattern, back_pattern)
SELECT note_type_id, template_index, name, front_pattern, back_pattern
FROM note_type_templates
WHERE note_type_id IN (SELECT id FROM note_types);

DROP TABLE note_type_templates;
ALTER TABLE note_type_templates_new RENAME TO note_type_templates;

-- 2. Rebuild cards with template_id (NOT NULL) instead of template_index.
CREATE TABLE cards_new (
    id INTEGER PRIMARY KEY,
    note_id INTEGER NOT NULL,
    deck_id INTEGER NOT NULL,
    template_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES note_type_templates(id) ON DELETE CASCADE
);

INSERT INTO cards_new (id, note_id, deck_id, template_id, created_at)
SELECT c.id, c.note_id, c.deck_id,
       (SELECT t.id
        FROM notes n
        JOIN note_type_templates t
            ON t.note_type_id = n.note_type_id AND t.ord = c.template_index
        WHERE n.id = c.note_id),
       c.created_at
FROM cards c;

DROP TABLE cards;
ALTER TABLE cards_new RENAME TO cards;

CREATE UNIQUE INDEX idx_cards_note_template ON cards(note_id, template_id);
CREATE INDEX idx_cards_deck ON cards(deck_id);

PRAGMA foreign_keys = ON;
