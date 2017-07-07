CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL
);
INSERT INTO user_new (id, username) SELECT id, username FROM user;
DROP TABLE user;
ALTER TABLE user_new RENAME to user;
