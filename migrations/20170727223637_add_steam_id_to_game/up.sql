CREATE TABLE game_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    steam_id INTEGER
);
INSERT INTO game_new (id, name) SELECT id, name FROM game;
DROP TABLE game;
ALTER TABLE game_new RENAME to game;
