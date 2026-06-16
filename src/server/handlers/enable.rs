use serenity::all::CommandInteraction;

use super::may_manage_guild;
use crate::{
    server::{AppState, Error, Message, Result},
    sql::enable_channel,
};

/// Enable the export command for a channel.
///
/// 1. Ensure the user has appropriate channel-management permission
/// 2. Add the `(guild, channel)` pair to the local DB
/// 3. Respond with success message
pub async fn enable(interaction: CommandInteraction, app_state: &AppState) -> Result<Message> {
    if !may_manage_guild(&interaction)? {
        // member has insufficient permissions to do this
        todo!("return an 'insufficient permissions' response")
    }

    let guild = interaction
        .guild_id
        .ok_or(Error::MalformedInput("no guild id"))?;

    let connection = app_state.local_db.lock().await;

    let enabled = enable_channel(
        &connection,
        guild,
        interaction.channel_id,
        interaction.user.id,
    )
    .await?;

    todo!("emit success message")
}
