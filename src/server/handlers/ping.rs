use crate::server::{Pong, Result};
use serenity::all::PingInteraction;

/// Immediately respond to a ping with a pong
pub async fn ping(_interation: PingInteraction) -> Result<Pong> {
    Ok(Pong)
}
