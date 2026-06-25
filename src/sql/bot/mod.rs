use rusqlite::{Connection, named_params};
use serenity::all::{ChannelId, GuildId, UserId};
use tokio::task::block_in_place;

use crate::sql::ToSqlInteger as _;

use super::Result;

/// enable a channel in the local database
///
/// Returns `true` if the channel was enabled, or `false` if it had already previously been enabled
pub async fn enable_channel(
    connection: &Connection,
    guild: GuildId,
    channel: ChannelId,
    actor: UserId,
) -> Result<bool> {
    block_in_place(|| {
        // OR IGNORE means if the (guild_id, channel_id) pair already existed, silently do nothing.
        let query = "INSERT OR IGNORE
            INTO enabled_channels (guild_id, channel_id, enabled_by)
            VALUES (:guild_id, :channel_id, :actor)";

        let mut stmt = connection.prepare_cached(query)?;
        let rows = stmt.execute(named_params! {
            ":guild_id": guild.to_sql(),
            ":channel_id": channel.to_sql(),
            ":actor": actor.to_sql(),
        })?;

        Ok(rows != 0)
    })
}

/// disable a channel in the local database
///
/// Returns `true` if the channel was disabled, or `false` if it had not been enabled in the first place
pub async fn disable_channel(
    connection: &Connection,
    guild: GuildId,
    channel: ChannelId,
) -> Result<bool> {
    block_in_place(|| {
        let query = "DELETE FROM enabled_channels
            WHERE guild_id = :guild_id
            AND channel_id = :channel_id";

        let mut stmt = connection.prepare_cached(query)?;
        let rows = stmt.execute(named_params! {
            ":guild_id": guild.to_sql(),
            ":channel_id": channel.to_sql(),
        })?;

        Ok(rows != 0)
    })
}

pub async fn channel_is_enabled(
    connection: &Connection,
    guild: GuildId,
    channel: ChannelId,
) -> Result<bool> {
    block_in_place(|| {
        let query = "
        SELECT EXISTS (
            SELECT 1 FROM enabled_channels
            WHERE guild_id = :guild_id
            AND channel_id = :channel_id
        )";

        let mut stmt = connection.prepare_cached(query)?;
        stmt.query_one(
            named_params! {
                ":guild_id": guild.to_sql(),
                ":channel_id": channel.to_sql(),
            },
            |row| row.get(0),
        )
        .map_err(Into::into)
    })
}
