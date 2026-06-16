use crate::{
    server::{AppState, Error, Message, Result, handlers::may_manage_guild},
    sql::disable_channel,
};
use serenity::all::{CommandInteraction, CreateInteractionResponseMessage};

/// Disable the export command for a channel.
///
/// 1. Ensure the user has appropriate channel-management permission
/// 2. Remove the `(guild, channel)` pair from the local DB
/// 3. Respond with success message
pub async fn disable(interaction: CommandInteraction, app_state: &AppState) -> Result<Message> {
    if !may_manage_guild(&interaction)? {
        // member has insufficient permissions to do this
        return Ok(CreateInteractionResponseMessage::new()
            .ephemeral(true)
            .content("permission denied")
            .into());
    }

    let guild = interaction
        .guild_id
        .ok_or(Error::MalformedInput("no guild id"))?;

    let connection = app_state.local_db.lock().await;

    let was_enabled = disable_channel(&connection, guild, interaction.channel_id).await?;
    let msg = if was_enabled {
        "DAAS has been disabled for this channel"
    } else {
        "DAAS was already disabled for this channel"
    };

    Ok(CreateInteractionResponseMessage::new()
        .ephemeral(!was_enabled)
        .content(msg)
        .into())
}
