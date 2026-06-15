mod cleanup;
mod disable;
mod enable;
mod export;
mod help;
mod ping;

use serenity::all::CommandInteraction;

pub use self::{
    cleanup::cleanup, disable::disable, enable::enable, export::export, help::help, ping::ping,
};
use super::{Error, Result};

/// Ensure that the user who created the interaction has "manage guild" permissions
fn may_manage_guild(interaction: &CommandInteraction) -> Result<bool> {
    let has_manage_guild_permission = interaction
        .member
        .as_ref()
        .ok_or(Error::MalformedInput("no interaction member"))?
        .permissions
        .ok_or(Error::MalformedInput("no member permissions"))?
        .manage_guild();
    Ok(has_manage_guild_permission)
}
