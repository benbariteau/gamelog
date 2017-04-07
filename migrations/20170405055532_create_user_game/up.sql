CREATE TABLE user_game (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    play_state TEXT NOT NULL,
    acquisition_date INTEGER NOT NULL,
    start_date INTEGER,
    beat_date INTEGER
);
