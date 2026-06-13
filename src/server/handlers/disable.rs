use crate::server::{Message, Result};
use serenity::all::CommandInteraction;

/// Disable the export command for a channel.
///
/// 1. Attempt to disable it by setting permissions or something, more thought required, 2s timeout.
/// 2. Respond with a message reporting either success or timeout.
pub async fn disable(_interation: CommandInteraction) -> Result<Message> {
    todo!()
}
