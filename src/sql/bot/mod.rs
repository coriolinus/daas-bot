use rusqlite::{Connection, named_params};
use serenity::all::{ChannelId, GuildId, UserId};
use tokio::task::block_in_place;

use super::Result;

trait ToSqlInteger: Sized {
    /// Transform `self` into a `u64`
    fn to_u64(self) -> u64;

    /// Transform `self` into a `i64` which sqlite can handle
    ///
    /// This is a pure binary transform; we don't care about the value produced, only
    /// that it fits into an `i64` which is a Sqlite `INTEGER` type.
    fn to_sql(self) -> i64 {
        i64::from_ne_bytes(self.to_u64().to_ne_bytes())
    }
}

macro_rules! impl_to_sql_integer_via_get {
    ($( $t:ty ),*) => {
        $(
            impl ToSqlInteger for $t {
                fn to_u64(self) -> u64 { self.get() }
            }
        )*
    };
}

impl_to_sql_integer_via_get!(ChannelId, GuildId, UserId);

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
