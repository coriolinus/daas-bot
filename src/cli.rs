use std::path::PathBuf;

use log::LevelFilter;
use serenity::all::{ApplicationId, GuildId};

/// DAAS-Bot lets you use Discord As A Spreadsheet.
#[derive(Debug, clap::Parser)]
#[command(max_term_width = 75)]
pub struct Args {
    /// Discord Application Id
    ///
    /// This is sourced from "General Information" in the developer portal
    ///
    /// Always required
    #[arg(long, env, required = true)]
    pub application_id: ApplicationId,

    /// Discord Bot Public Key
    ///
    /// This is sourced from "General Information" in the developer portal
    ///
    /// Required unless using `--register`.
    #[arg(
        long,
        env,
        default_value_t = Default::default(),
        hide_default_value = true,
        required_unless_present = "register",
    )]
    pub public_key: String,

    /// Discord Bot Token
    ///
    /// This is sourced from "Bot" in the devloper portal
    ///
    /// Required unless using `--register`.
    ///
    /// This is sensitive! Treat it carefully.
    #[arg(
        long,
        env,
        default_value_t = Default::default(),
        hide_default_value = true,
        required_unless_present = "register",
        hide_env_values = true,
    )]
    pub bot_token: String,

    /// Register this bot's commands with Discord
    ///
    /// This manually updates Discord with the slash commands supported by this bot,
    /// then exits. See also `--autoregister` which sends the update and then continues
    /// execution.
    ///
    /// Note that global registration can take up to half an hour to propagate to all
    /// servers. The `--guild` argument can narrow this to a single guild, which happens
    /// immediately. This is most helpful for testing.
    #[arg(long, conflicts_with = "autoregister")]
    pub register: bool,

    /// Register this bot's commands before continuing to execute.
    ///
    /// As `--register`, but does not immediately exit after sending the update.
    #[arg(long)]
    pub autoregister: bool,

    /// Only register this bot's commands for a particular guild.
    ///
    /// This is mostly a debugging tool.
    ///
    /// Has no effect unless `--register` or `--autoregister` is set.
    #[arg(
        long,
        value_name = "GUILD_ID",
        requires = "register",
        requires = "autoregister"
    )]
    pub guild: Option<GuildId>,

    /// The port on which the server should listen.
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Path to the database file.
    ///
    /// This is the local database, which mainly keeps track of which channels
    /// have been enabled.
    ///
    /// Required unless using `--register`.
    #[arg(long, env, required = false, required_unless_present = "register")]
    pub database_path: PathBuf,

    /// Log level
    ///
    /// Logs below this level are filtered out.
    #[arg(short, long, default_value_t = LevelFilter::Info)]
    pub log_level: LevelFilter,
}
