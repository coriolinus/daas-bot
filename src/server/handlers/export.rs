use crate::{
    server::{AppState, Defer, Error, Result},
    sql::channel_is_enabled,
};
use serenity::all::CommandInteraction;

/// Export the data gathered in this channel to an sqlite file.
///
/// See the README for parseable message format and data format.
///
/// 1. verify that this is an enabled channel.
/// 2. kick off an async task to export the data into sql (see readme)
/// 3. while that's running, respond with this defer message
pub async fn export(interaction: CommandInteraction, app_state: &AppState) -> Result<Defer> {
    let guild = interaction
        .guild_id
        .ok_or(Error::MalformedInput("no guild id"))?;

    let connection = app_state.local_db.lock().await;

    if !channel_is_enabled(&connection, guild, interaction.channel_id).await? {
        todo!("rethink the reurn type, we don't want a `Defer` here, we want an immediate message");
        todo!("return a message that the channel has not been enabled");
    }

    todo!(
        "launch the task to collect the message data (no await!), then return a blank defer message"
    )
}
