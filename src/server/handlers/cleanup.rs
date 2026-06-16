use crate::server::{AppState, Defer, Result};
use serenity::all::CommandInteraction;

/// Delete all but the most recent export from this server.
///
/// 1. Launch an async task to actually accomplish that.
/// 2. While that's running, return this response.
pub async fn cleanup(_interation: CommandInteraction, app_state: &AppState) -> Result<Defer> {
    todo!()
}
