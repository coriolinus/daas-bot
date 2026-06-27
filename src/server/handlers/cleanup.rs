use either::Either;
use log::debug;
use serenity::all::{CommandInteraction, CreateInteractionResponseMessage};

use crate::{
    server::{AppState, Defer, Error, Message, Result},
    sql::channel_is_enabled,
};

/// Delete all but the most recent export from this server.
///
/// 1. Launch an async task to actually accomplish that.
/// 2. While that's running, return this response.
pub async fn cleanup(
    interaction: CommandInteraction,
    app_state: &AppState,
) -> Result<Either<Message, Defer>> {
    debug!("handling cleanup interaction");
    let guild = interaction
        .guild_id
        .ok_or(Error::MalformedInput("no guild id"))?;

    let connection = app_state.local_db.clone().lock_owned().await;

    if !channel_is_enabled(connection, guild, interaction.channel_id).await? {
        return Ok(Either::Left(Message::from(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("permission denied"),
        )));
    }

    Ok(Either::Left(Message::from(
        CreateInteractionResponseMessage::new()
            .ephemeral(true)
            .content("🚧 `/daas cleanup` is not yet implemented. ask a channel admin to delete old messages.")
    )))
}
