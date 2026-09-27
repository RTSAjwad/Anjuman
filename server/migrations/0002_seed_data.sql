-- Anjuman seed data (dev/test). Idempotent.
--
-- All passwords are hashed with Argon2id and are VALID:
--   teacher@school1.com / teach123
--   admin@school1.com   / admin123
--   student@school1.com / stud123
--
-- The fixed `1700000000` epoch (2023-11-14) mirrors the old SQLite seed.

--------------------------------------------------------------------
-- System school (owns the global default preset)
--------------------------------------------------------------------

INSERT INTO schools (id, name, created_at)
VALUES (0, 'System', 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

--------------------------------------------------------------------
-- Global default deck-options preset (id 0, owned by system school)
--------------------------------------------------------------------

INSERT INTO deck_options
    (id, school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, created_at)
VALUES
    (0, 0, 'Default', 0.9, FALSE, FALSE, FALSE, 20, 200, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

INSERT INTO deck_option_steps (options_id, kind, step_index, seconds)
VALUES
    (0, 'learning',   0, 60),
    (0, 'learning',   1, 600),
    (0, 'relearning', 0, 600)
ON CONFLICT (options_id, kind, step_index) DO NOTHING;

--------------------------------------------------------------------
-- Dev school
--------------------------------------------------------------------

INSERT INTO schools (id, name, created_at)
VALUES (1, 'Springfield High', 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

--------------------------------------------------------------------
-- Users
--------------------------------------------------------------------

INSERT INTO users (id, school_id, email, password_hash, role, first_name, last_name, created_at)
VALUES
    (1, 1, 'teacher@school1.com', '$argon2id$v=19$m=19456,t=2,p=1$7ArQkDZAbK6WWtSMDD3swg$KzG8ucoLFw86POIGm9cZotQJiG/vM+R+Drsz19SgAvo', 'teacher', 'Alice', 'Johnson', 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second'),
    (2, 1, 'admin@school1.com',   '$argon2id$v=19$m=19456,t=2,p=1$FtKF9MeZcvxkgd1AnsG35Q$5mh2ZTwlqufppZ8XXJwjnMvvMW45/1EOq0l/iy8Kuaw', 'admin',   'Bob',   'Williams', 'epoch'::TIMESTAMPTZ + 1700000001 * INTERVAL '1 second'),
    (3, 1, 'student@school1.com', '$argon2id$v=19$m=19456,t=2,p=1$Q2CRjpZ1fVPtBf5SUR8Klg$e5XdI09f5EKMj4GMGCojXrWirexuuA2nwiI1QGPaNUQ', 'student', 'Charlie', 'Smith', 'epoch'::TIMESTAMPTZ + 1700000002 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

--------------------------------------------------------------------
-- Class + membership
--------------------------------------------------------------------

INSERT INTO classes (id, school_id, name, description, created_by, created_at)
VALUES (1, 1, 'Biology 101', 'Introduction to biology', 1, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

INSERT INTO class_members (class_id, user_id, role, joined_at)
VALUES (1, 3, 'student', 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (class_id, user_id) DO NOTHING;

--------------------------------------------------------------------
-- Built-in note types
--------------------------------------------------------------------

INSERT INTO note_types (id, school_id, name, field_names, sort_field, created_by, created_at)
VALUES
    (1, 1, 'Basic', '["Front","Back"]', 'Front', 1, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second'),
    (2, 1, 'Basic (and reversed)', '["Front","Back"]', 'Front', 1, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

INSERT INTO note_type_templates (note_type_id, ord, name, front_pattern, back_pattern)
VALUES
    (1, 0, 'Card 1', '{{Front}}', '{{Front}}<hr>{{Back}}'),
    (2, 0, 'Card 1', '{{Front}}', '{{Front}}<hr>{{Back}}'),
    (2, 1, 'Card 2', '{{Back}}', '{{Back}}<hr>{{Front}}')
ON CONFLICT (note_type_id, ord) DO NOTHING;

--------------------------------------------------------------------
-- Dev deck + class link
--------------------------------------------------------------------

INSERT INTO decks (id, school_id, title, description, created_by, parent_id, created_at)
VALUES (1, 1, 'Biology 101', 'Core biology deck', 1, NULL, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

INSERT INTO deck_classes (deck_id, class_id, added_at)
VALUES (1, 1, 'epoch'::TIMESTAMPTZ + 1700000000 * INTERVAL '1 second')
ON CONFLICT (deck_id, class_id) DO NOTHING;

--------------------------------------------------------------------
-- Notes & Cards (10 basic biology questions)
--------------------------------------------------------------------

INSERT INTO notes (id, note_type_id, fields_json, created_at)
VALUES
    (1,  1, '{"Front":"What is the powerhouse of the cell?","Back":"Mitochondria"}', 'epoch'::TIMESTAMPTZ + 1700000100 * INTERVAL '1 second'),
    (2,  1, '{"Front":"What is the process by which plants make food?","Back":"Photosynthesis"}', 'epoch'::TIMESTAMPTZ + 1700000101 * INTERVAL '1 second'),
    (3,  1, '{"Front":"What gas do plants absorb from the atmosphere?","Back":"Carbon dioxide"}', 'epoch'::TIMESTAMPTZ + 1700000102 * INTERVAL '1 second'),
    (4,  1, '{"Front":"What is the basic unit of life?","Back":"The cell"}', 'epoch'::TIMESTAMPTZ + 1700000103 * INTERVAL '1 second'),
    (5,  1, '{"Front":"What organelle contains genetic material?","Back":"Nucleus"}', 'epoch'::TIMESTAMPTZ + 1700000104 * INTERVAL '1 second'),
    (6,  1, '{"Front":"What is the jelly-like substance inside a cell?","Back":"Cytoplasm"}', 'epoch'::TIMESTAMPTZ + 1700000105 * INTERVAL '1 second'),
    (7,  1, '{"Front":"Which organelle produces proteins?","Back":"Ribosomes"}', 'epoch'::TIMESTAMPTZ + 1700000106 * INTERVAL '1 second'),
    (8,  1, '{"Front":"What is the cell membrane made of?","Back":"Phospholipid bilayer"}', 'epoch'::TIMESTAMPTZ + 1700000107 * INTERVAL '1 second'),
    (9,  1, '{"Front":"What molecule carries genetic information?","Back":"DNA"}', 'epoch'::TIMESTAMPTZ + 1700000108 * INTERVAL '1 second'),
    (10, 1, '{"Front":"What organelle is responsible for packaging proteins?","Back":"Golgi apparatus"}', 'epoch'::TIMESTAMPTZ + 1700000109 * INTERVAL '1 second')
ON CONFLICT (id) DO NOTHING;

INSERT INTO cards (id, note_id, deck_id, template_id, created_at)
SELECT v.id, v.note_id, v.deck_id, t.id, v.created_at
FROM (
    VALUES
        (1,  1,  1, 'epoch'::TIMESTAMPTZ + 1700000100 * INTERVAL '1 second'),
        (2,  2,  1, 'epoch'::TIMESTAMPTZ + 1700000101 * INTERVAL '1 second'),
        (3,  3,  1, 'epoch'::TIMESTAMPTZ + 1700000102 * INTERVAL '1 second'),
        (4,  4,  1, 'epoch'::TIMESTAMPTZ + 1700000103 * INTERVAL '1 second'),
        (5,  5,  1, 'epoch'::TIMESTAMPTZ + 1700000104 * INTERVAL '1 second'),
        (6,  6,  1, 'epoch'::TIMESTAMPTZ + 1700000105 * INTERVAL '1 second'),
        (7,  7,  1, 'epoch'::TIMESTAMPTZ + 1700000106 * INTERVAL '1 second'),
        (8,  8,  1, 'epoch'::TIMESTAMPTZ + 1700000107 * INTERVAL '1 second'),
        (9,  9,  1, 'epoch'::TIMESTAMPTZ + 1700000108 * INTERVAL '1 second'),
        (10, 10, 1, 'epoch'::TIMESTAMPTZ + 1700000109 * INTERVAL '1 second')
) AS v(id, note_id, deck_id, created_at)
CROSS JOIN LATERAL (
    SELECT id FROM note_type_templates WHERE note_type_id = 1 AND ord = 0 LIMIT 1
) AS t
ON CONFLICT (id) DO NOTHING;
