-- Per-user scheduling preferences (Anki's user-level options).
--
-- These are NOT deck/preset options — they apply to a single user across all
-- decks. The learn-ahead limit lets a learning/relearning card whose next step
-- is due within this many seconds be studied early, as a fallback once all
-- actually-due cards are exhausted. Anki's default is 20 minutes.

CREATE TABLE user_preferences (
    user_id INTEGER PRIMARY KEY,
    learn_ahead_seconds INTEGER NOT NULL DEFAULT 1200,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
