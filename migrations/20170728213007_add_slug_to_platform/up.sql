CREATE TABLE platform_new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    short_name TEXT NOT NULL,
    slug TEXT NOT NULL
);
DROP TABLE platform;
ALTER TABLE platform_new RENAME to platform;
