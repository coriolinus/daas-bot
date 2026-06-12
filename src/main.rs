mod cli;
mod error;
mod register;

use anyhow::Result;
use clap::Parser as _;

use cli::Args;
use register::register;
use serenity::all::Http;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let http = Http::new(&args.bot_token);
    http.set_application_id(args.application_id);

    if args.register || args.autoregister {
        register(&http, args.guild).await?;

        if args.register {
            return Ok(());
        }
    }

    Ok(())
}
