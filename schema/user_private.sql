CREATE TABLE user_private (
    id INTEGER PRIMARY KEY,
    user_id INTEGER UNIQUE,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL
);
