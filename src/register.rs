use anyhow::{Context as _, Result};
use serenity::all::{
    Command, CommandOptionType, CreateCommand, CreateCommandOption, GuildId, Http,
    InteractionContext,
};

pub async fn register(http: &Http, guild: Option<GuildId>) -> Result<Command> {
    let command = CreateCommand::new("daas")
        .description("DAAS Bot Commands")
        .add_context(InteractionContext::Guild)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "enable",
            "enable daas bot to parse and export the current channel",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "disable",
            "disable daas bot operation in the current channel",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "export",
            "analyze the current channel and export its contents as a sqlite file",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "cleanup",
            "remove all but the final export message from this channel",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "help",
            "emit a message with usage instructions for daas bot",
        ));

    match guild {
        Some(guild) => http
            .create_guild_command(guild, &command)
            .await
            .context(format!("creating guild-specific /daas command in {guild}")),
        None => http
            .create_global_command(&command)
            .await
            .context("creating global /daas command"),
    }
}
