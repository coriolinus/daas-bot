//! Export a Discord conversation into a Sqlite database.
//!
//! This module contains the actual implementation driving that export.

mod error;
mod item;

use std::sync::Arc;

use rusqlite::Connection;
use serenity::all::{ChannelId, CommandInteraction, Http, Message, MessagePagination};
use tokio::{select, spawn, sync::mpsc};

use crate::sql::export;

pub(crate) use self::{
    error::{Error, Result},
    item::{Item, ItemWithMetadata},
};

/// Container type which holds fundamental data about the command which launched this export,
/// a handle to the Http client, and the Sqlite connection.
///
/// This struct acts as a receiver onto which other
#[derive(Debug)]
pub struct Exporter {
    interaction: CommandInteraction,
    http: Arc<Http>,
    connection: Connection,
}

impl Exporter {
    pub async fn new(interaction: CommandInteraction, http: Arc<Http>) -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(crate::sql::Error::from)?;
        export::apply_schema(&connection).await?;

        Ok(Self {
            interaction,
            http,
            connection,
        })
    }

    /// Drive this export to completion.
    pub async fn drive(mut self) -> Result<Vec<u8>> {
        let (item_tx, mut item_rx) = mpsc::channel(128);
        let (messages_tx, messages_rx) = mpsc::channel(2);
        let (error_tx, mut error_rx) = mpsc::channel(1);

        spawn(Self::process_messages(messages_rx, item_tx));
        spawn(Self::fetch_message_pages(
            self.http.clone(),
            self.interaction.channel_id,
            messages_tx,
            error_tx,
        ));

        loop {
            select! {
                // If the fetch pages handle is finished then the error receiver will always promptly
                // produce a pointless `None` value, which is not helpful. We can just ignore that case
                // because we want to keep processing until the items are processed anyway.
                Some(err) = error_rx.recv() => return Err(err),
                maybe_item = item_rx.recv() => match maybe_item {
                    Some(item) => {
                        let item_id = export::add_item(&mut self.connection, &item).await?;
                        todo!("send the item id to another task to get the reactions")
                    }
                    None => {
                        // item channel closed means message processor completed
                        break;
                    }
                }
                // TODO: add a branch here to receive the reactions and add them to the database
                // and at the same time break the loop when the reaction sender completes not on maybe_item
            }
        }

        todo!("export the database")
    }

    /// Fetch a batch of messages from Discord and send them for processing.
    ///
    /// This iterates as fast as possible over the channel's pages of messages,
    /// sending each page over the channel and then fetching the next page.
    ///
    /// It's a good idea to keep the `msg_tx` capacity low (~2) to provide backpressure.
    async fn fetch_message_pages(
        http: Arc<Http>,
        channel_id: ChannelId,
        msg_tx: mpsc::Sender<Vec<Message>>,
        err_tx: mpsc::Sender<Error>,
    ) {
        let mut before = None;

        loop {
            // https://docs.discord.com/developers/resources/message#get-channel-messages:
            // > limit?	integer	Max number of messages to return (1-100)
            const GET_MESSAGES_LIMIT: Option<u8> = Some(100);

            match http
                .get_messages(
                    channel_id,
                    before.map(MessagePagination::Before),
                    GET_MESSAGES_LIMIT,
                )
                .await
                .map_err(Error::Http)
                // https://docs.discord.com/developers/resources/message#get-channel-messages:
                // > Returns an array of message objects from newest to oldest
                .inspect(|messages| before = messages.last().map(|message| message.id))
            {
                Ok(messages) => {
                    if messages.is_empty() {
                        // we've exhausted the messages in the channel
                        break;
                    }
                    if msg_tx.send(messages).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    if err_tx.send(err).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    /// Process a batch of messages: parse them into `ItemWithMetadata` instances and send each to the relevant sender
    async fn process_messages(
        mut msg_rx: mpsc::Receiver<Vec<Message>>,
        item_tx: mpsc::Sender<ItemWithMetadata>,
    ) {
        while let Some(messages) = msg_rx.recv().await {
            for msg in messages {
                if let Ok(item) = msg.try_into()
                    && item_tx.send(item).await.is_err()
                {
                    break;
                }
            }
        }
    }
}
