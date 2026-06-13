use crate::server::{Defer, Result};
use serenity::all::CommandInteraction;

/// Export the data gathered in this channel to an sqlite file.
///
/// See the README for parseable message format and data format.
///
/// 1. verify that this is an enabled channel.
/// 2. kick off an async task to export the data into sql (see readme)
/// 3. while that's running, respond with this defer message
pub async fn export(_interation: CommandInteraction) -> Result<Defer> {
    todo!()
}
