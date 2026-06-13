use crate::server::{Message, Result};
use serenity::all::CommandInteraction;

/// Immediately return a help message giving an overview of what the commands are and what each does.
pub async fn help(_interation: CommandInteraction) -> Result<Message> {
    todo!()
}
