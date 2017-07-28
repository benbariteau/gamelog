CREATE TABLE platform_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    short_name TEXT NOT NULL
);
INSERT INTO platform_new (id, name, short_name) SELECT id, name, short_name FROM platform;
DROP TABLE platform;
ALTER TABLE platform_new RENAME to platform;
