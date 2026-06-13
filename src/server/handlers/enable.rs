use crate::server::{Message, Result};
use serenity::all::CommandInteraction;

/// Enable the export command for a channel.
///
/// 1. Attempt to enable it by setting permissions or something, more thought required, 2s timeout.
/// 2. Respond with a message reporting either success or timeout.
pub async fn enable(_interation: CommandInteraction) -> Result<Message> {
    todo!()
}
