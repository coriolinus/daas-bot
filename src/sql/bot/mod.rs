use rusqlite::{Connection, named_params};
use serenity::all::{ChannelId, GuildId, UserId};
use tokio::{sync::OwnedMutexGuard, task::spawn_blocking};

use crate::sql::ToSqlInteger as _;

use super::Result;

refinery::embed_migrations!("src/sql/bot/migrations");

/// enable a channel in the local database
///
/// Returns `true` if the channel was newly enabled, or `false` if it had already previously been enabled
pub async fn enable_channel(
    connection: OwnedMutexGuard<Connection>,
    guild: GuildId,
    channel: ChannelId,
    actor: UserId,
) -> Result<bool> {
    spawn_blocking(move || {
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
    .await
    .map_err(Into::into)
    .flatten()
}

/// disable a channel in the local database
///
/// Returns `true` if the channel was disabled, or `false` if it had not been enabled in the first place
pub async fn disable_channel(
    connection: OwnedMutexGuard<Connection>,
    guild: GuildId,
    channel: ChannelId,
) -> Result<bool> {
    spawn_blocking(move || {
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
    .await
    .map_err(Into::into)
    .flatten()
}

pub async fn channel_is_enabled(
    connection: OwnedMutexGuard<Connection>,
    guild: GuildId,
    channel: ChannelId,
) -> Result<bool> {
    spawn_blocking(move || {
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
    .await
    .map_err(Into::into)
    .flatten()
}
