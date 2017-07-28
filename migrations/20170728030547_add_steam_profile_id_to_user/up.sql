CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    steam_id TEXT
);
INSERT INTO user_new (id, username, email) SELECT id, username, email FROM user;
DROP TABLE user;
ALTER TABLE user_new RENAME to user;
