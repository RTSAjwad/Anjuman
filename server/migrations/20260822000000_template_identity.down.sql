-- Reverse the template-identity refactor (best effort; matches the table-rebuild up migration).

PRAGMA foreign_keys = OFF;

-- Rebuild cards with positional template_index (derived from template ord).
CREATE TABLE cards_old (
    id INTEGER PRIMARY KEY,
    note_id INTEGER NOT NULL,
    deck_id INTEGER NOT NULL,
    template_index INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE
);

INSERT INTO cards_old (id, note_id, deck_id, template_index, created_at)
SELECT c.id, c.note_id, c.deck_id, t.ord, c.created_at
FROM cards c
JOIN note_type_templates t ON t.id = c.template_id;

DROP TABLE cards;
ALTER TABLE cards_old RENAME TO cards;

CREATE UNIQUE INDEX idx_cards_note_template ON cards(note_id, template_index);
CREATE INDEX idx_cards_deck ON cards(deck_id);

-- Rebuild note_type_templates without id (ord becomes template_index).
CREATE TABLE note_type_templates_old (
    note_type_id INTEGER NOT NULL,
    template_index INTEGER NOT NULL,
    name TEXT NOT NULL,
    front_pattern TEXT NOT NULL,
    back_pattern TEXT NOT NULL,
    PRIMARY KEY (note_type_id, template_index),
    FOREIGN KEY (note_type_id) REFERENCES note_types(id) ON DELETE CASCADE
);

INSERT INTO note_type_templates_old (note_type_id, template_index, name, front_pattern, back_pattern)
SELECT note_type_id, ord, name, front_pattern, back_pattern FROM note_type_templates;

DROP TABLE note_type_templates;
ALTER TABLE note_type_templates_old RENAME TO note_type_templates;

PRAGMA foreign_keys = ON;
