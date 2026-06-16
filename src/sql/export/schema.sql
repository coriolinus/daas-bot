PRAGMA foreign_keys = ON;

-- Items
CREATE TABLE items (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    posted_by   INTEGER NOT NULL REFERENCES users(id),
    title       TEXT    NOT NULL,
    description TEXT    NOT NULL,
    created     TEXT    NOT NULL, -- time of discord message creation
    edited      TEXT        NULL  -- time of discord message update
);

-- Tags
CREATE TABLE tags (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    description TEXT    NOT NULL,
);

-- Tag associations
CREATE TABLE tag_associations (
    item_id     INTEGER NOT NULL REFERENCES items(id),
    tag_id      INTEGER NOT NULL REFERENCES tags(id),
    PRIMARY KEY (item_id, tag_id)
);

-- Users
CREATE TABLE users (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    display_name TEXT    NOT NULL
);

CREATE TRIGGER users_updated
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE users SET updated = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
    WHERE id = OLD.id;
END;

-- Categories
CREATE TABLE categories (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    emoji   TEXT    NOT NULL,
);

-- Votes
CREATE TABLE votes (
    item_id     INTEGER NOT NULL REFERENCES items(id),
    user_id     INTEGER NOT NULL REFERENCES users(id),
    category_id INTEGER NOT NULL REFERENCES categories(id),
    created     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    PRIMARY KEY (item_id, user_id, category_id)
);
