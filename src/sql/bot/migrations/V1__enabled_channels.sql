CREATE TABLE enabled_channels (
    guild_id    INTEGER NOT NULL,
    channel_id  INTEGER NOT NULL,
    enabled_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    enabled_by  INTEGER NOT NULL,
    PRIMARY KEY (guild_id, channel_id)
);
