mod cli;
mod export;
mod register;
mod server;
mod sql;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use log::{LevelFilter, debug};
use serenity::all::Http;

use cli::Args;
use register::register;
use simplelog::{ConfigBuilder, TermLogger};

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.log_level != LevelFilter::Off {
        TermLogger::init(
            args.log_level,
            ConfigBuilder::default()
                .add_filter_ignore_str("serenity::http")
                .build(),
            Default::default(),
            Default::default(),
        )
        .context("initializing default logger")?;
    }

    let http = Http::new(&args.bot_token);
    http.set_application_id(args.application_id);
    debug_assert!(http.ratelimiter.is_some(), "we must have a rate limiter");

    if args.register || args.autoregister {
        register(&http, args.guild).await?;

        if args.register {
            return Ok(());
        }
    }

    debug!("parsed initial arguments; running the server");
    server::run(args, http).await
}
