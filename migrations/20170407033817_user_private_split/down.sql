CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL
);
INSERT INTO user_new (id, username, password_hash, salt)
SELECT user.id, user.username, user_private.password_hash, user_private.salt
FROM user JOIN user_private ON user.id = user_private.user_id;
DROP TABLE user;
DROP TABLE user_private;
ALTER TABLE user_new RENAME TO user;
