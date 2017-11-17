CREATE TABLE user_game_new (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    play_state TEXT NOT NULL,
    acquisition_date INTEGER NOT NULL,
    start_date INTEGER,
    beat_date INTEGER
);
INSERT INTO user_game_new (id, game_id, user_id, play_state, acquisition_date, start_date, beat_date) SELECT id, game_id, user_id, play_state, acquisition_date, start_date, beat_date FROM user_game;
DROP TABLE user_game;
ALTER TABLE user_game_new RENAME to user_game;
