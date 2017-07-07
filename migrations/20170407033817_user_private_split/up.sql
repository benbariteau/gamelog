CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL
);
CREATE TABLE user_private (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL
);
INSERT INTO user_new (id, username) SELECT id, username FROM user;
INSERT INTO user_private (user_id, password_hash, salt) SELECT id, password_hash, salt FROM user;
DROP TABLE user;
ALTER TABLE user_new RENAME to user;
