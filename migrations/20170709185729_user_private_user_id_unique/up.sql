CREATE TABLE user_private_new (
    id INTEGER PRIMARY KEY,
    user_id INTEGER UNIQUE,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL
);
INSERT INTO user_private_new (id, user_id, password_hash, salt) SELECT id, user_id, password_hash, salt FROM user_private;
DROP TABLE user_private;
ALTER TABLE user_private_new RENAME to user_private;
