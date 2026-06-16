mod cli;
mod export;
mod register;
mod server;
mod sql;

use anyhow::Result;
use clap::Parser as _;

use cli::Args;
use register::register;
use serenity::all::Http;

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let http = Http::new(&args.bot_token);
    http.set_application_id(args.application_id);
    debug_assert!(http.ratelimiter.is_some(), "we must have a rate limiter");

    if args.register || args.autoregister {
        register(&http, args.guild).await?;

        if args.register {
            return Ok(());
        }
    }

    server::run(args, http).await
}
