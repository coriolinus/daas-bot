use log::debug;
use serenity::all::{CommandInteraction, CreateInteractionResponseMessage};

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
    debug!("handling enable interaction");
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

    let already_enabled = enable_channel(
        &connection,
        guild,
        interaction.channel_id,
        interaction.user.id,
    )
    .await?;

    let msg = if already_enabled {
        "DAAS was already enabled for this channel"
    } else {
        "DAAS enabled for this channel. Run `/daas export` to perform an export."
    };
    Ok(CreateInteractionResponseMessage::new()
        .ephemeral(already_enabled)
        .content(msg)
        .into())
}
