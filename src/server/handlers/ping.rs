use log::info;
use serenity::all::PingInteraction;

use crate::server::{Pong, Result};

/// Immediately respond to a ping with a pong
pub async fn ping(_interation: PingInteraction) -> Result<Pong> {
    info!("handling ping interaction");
    Ok(Pong)
}
