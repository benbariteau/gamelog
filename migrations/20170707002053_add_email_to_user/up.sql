CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE
);
INSERT INTO user_new (id, username, email) SELECT id, username, (username || "@gamelog.zone") FROM user;
DROP TABLE user;
ALTER TABLE user_new RENAME to user;
