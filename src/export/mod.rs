mod error;

use std::sync::Arc;

use rusqlite::Connection;
use serenity::all::{CommandInteraction, Http, MessagePagination};
use tokio::{
    spawn,
    sync::{Mutex, mpsc},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;

pub use self::error::{Error, Result};

#[derive(Debug)]
pub struct Exporter {
    interaction: CommandInteraction,
    http: Arc<Http>,
    connection: Mutex<Connection>,
    errors_rx: mpsc::Receiver<Error>,
    cancel: CancellationToken,
}

impl Exporter {
    pub fn new(interaction: CommandInteraction, http: Arc<Http>) -> Result<Self> {
        let connection = Connection::open_in_memory()?.into();

        todo!("write schema to connection");

        let (errors_tx, errors_rx) = mpsc::channel(1);
        let cancel = CancellationToken::new();

        let exporter = Self {
            interaction,
            http,
            connection,
            errors_rx,
            cancel,
        };

        spawn(exporter.fetch_messages(errors_tx, None));

        Ok(exporter)
    }

    /// Fetch a batch of messages from Discord and add them to the connection.
    //
    // This function exists for error-handling and cancelation-checking purposese.
    async fn fetch_messages(
        &self,
        errors_tx: mpsc::Sender<Error>,
        target: Option<MessagePagination>,
    ) {
        if self.cancel.is_cancelled() {
            return;
        }

        if let Err(e) = self.try_fetch_messages(errors_tx.clone(), target).await {
            self.cancel.cancel();
            let _ = errors_tx.send(e).await; // we don't care if the send errors; means the channel is closed.
        }
    }

    async fn try_fetch_messages(
        &self,
        errors_tx: mpsc::Sender<Error>,
        target: Option<MessagePagination>,
    ) -> Result<(), Error> {
        let messages = self
            .http
            .get_messages(self.interaction.channel_id, target, None)
            .await
            .map_err(Error::Http)?;

        todo!("handle messages");
        todo!("look for more messages")
    }
}
