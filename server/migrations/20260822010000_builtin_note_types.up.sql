-- Seed the second built-in note type: "Basic (and reversed)".
-- Idempotent; built-ins mirror the "Basic" seed (owned by the dev school 1).

INSERT OR IGNORE INTO note_types (id, school_id, name, field_names, sort_field, created_by, created_at)
VALUES (2, 1, 'Basic (and reversed)', '["Front","Back"]', 'Front', 1, '1700000000');

INSERT OR IGNORE INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern)
VALUES
    (2, 0, 'Card 1', '{{Front}}', '{{Front}}<hr>{{Back}}'),
    (2, 1, 'Card 2', '{{Back}}', '{{Back}}<hr>{{Front}}');
