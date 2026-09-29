-- Selection-only deck options (US-2.14): audio, on-screen timer, auto-advance.
--
-- These have no server-side behaviour — the server only persists and returns
-- the selection; the client (stage 4) performs the behaviour. Defaults match
-- Anki's in-app defaults (booleans off except "wait for audio" on).

CREATE TYPE auto_advance_question_action AS ENUM ('show_answer', 'show_card');
CREATE TYPE auto_advance_answer_action AS ENUM (
    'bury_card', 'answer_again', 'answer_good', 'answer_hard', 'show_reminder'
);

ALTER TABLE deck_options
    ADD COLUMN show_on_screen_timer BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN stop_timer_on_answer BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN dont_play_audio_automatically BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN skip_question_when_replaying_answer BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN auto_advance_seconds_show_question DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    ADD COLUMN auto_advance_seconds_show_answer DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    ADD COLUMN auto_advance_wait_for_audio BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN auto_advance_question_action auto_advance_question_action NOT NULL DEFAULT 'show_answer',
    ADD COLUMN auto_advance_answer_action auto_advance_answer_action NOT NULL DEFAULT 'bury_card';
