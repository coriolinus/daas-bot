use log::info;
use serenity::all::CommandInteraction;

use crate::server::{AppState, Defer, Result};

/// Delete all but the most recent export from this server.
///
/// 1. Launch an async task to actually accomplish that.
/// 2. While that's running, return this response.
pub async fn cleanup(_interation: CommandInteraction, _app_state: &AppState) -> Result<Defer> {
    info!("handling cleanup interaction");
    todo!()
}
