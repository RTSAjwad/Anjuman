-- Reverse the step normalisation.
ALTER TABLE deck_options ADD COLUMN learning_steps TEXT NOT NULL DEFAULT '60,600';
ALTER TABLE deck_options ADD COLUMN relearning_steps TEXT NOT NULL DEFAULT '600';

-- Re-materialise the default preset's steps as CSV (best effort; real presets'
-- steps would be reconstructed from deck_option_steps before dropping).
UPDATE deck_options SET learning_steps = '60,600', relearning_steps = '600' WHERE id = 0;

DROP TABLE deck_option_steps;

-- Remove the seeded system school and default preset.
DELETE FROM deck_options WHERE id = 0;
DELETE FROM schools WHERE id = 0;
