//! Export a Discord conversation into a Sqlite database.
//!
//! This module contains the actual implementation driving that export.

mod error;
mod item;
mod vote;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use rusqlite::Connection;
use serenity::all::{
    ChannelId, CommandInteraction, Http, Message, MessageId, MessagePagination, ReactionType,
};
use tempfile::NamedTempFile;
use tokio::{select, spawn, sync::mpsc, task::block_in_place};

use crate::sql::export;

pub(crate) use self::{
    error::{Error, Result},
    item::ItemWithMetadata,
    vote::Vote,
};

struct ReactionRequest {
    channel_id: ChannelId,
    message_id: MessageId,
    reaction_type: ReactionType,
}

impl ReactionRequest {
    fn new(message: &Message, reaction_type: ReactionType) -> Self {
        Self {
            channel_id: message.channel_id,
            message_id: message.id,
            reaction_type,
        }
    }
}

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
        const HIGH_BUFFER_SIZE: usize = 128;
        const REACTION_PROCESSING_TASKS: usize = 8;

        let (message_pages_tx, message_pages_rx) = mpsc::channel(2); // provide backpressure
        let (error_tx, mut error_rx) = mpsc::channel(1);
        let (item_tx, mut item_rx) = mpsc::channel(HIGH_BUFFER_SIZE);
        let (reaction_tx, reaction_rx) = async_channel::bounded(HIGH_BUFFER_SIZE);
        let (votes_tx, votes_rx) = mpsc::channel(HIGH_BUFFER_SIZE);
        let (persisted_items_tx, persisted_items_rx) = mpsc::channel(2);
        let (persistable_votes_tx, mut persistable_votes_rx) = mpsc::channel(HIGH_BUFFER_SIZE);

        spawn(fetch_message_pages(
            self.http.clone(),
            self.interaction.channel_id,
            message_pages_tx,
            error_tx.clone(),
        ));
        spawn(process_messages(message_pages_rx, item_tx, reaction_tx));
        for _ in 0..REACTION_PROCESSING_TASKS {
            spawn(fetch_reaction_users(
                self.http.clone(),
                reaction_rx.clone(),
                votes_tx.clone(),
                error_tx.clone(),
            ));
        }
        spawn(hold_votes_for_relevant_item_persistence(
            persisted_items_rx,
            votes_rx,
            persistable_votes_tx,
        ));

        loop {
            select! {
                Some(err) = error_rx.recv() => return Err(err),
                Some(item) = item_rx.recv() =>  {
                    export::add_item(&mut self.connection, &item).await?;
                    persisted_items_tx.send(item.message_id).await.map_err(Error::send_failed("persisted_items"))?;
                }
                Some(vote) = persistable_votes_rx.recv() => {
                    export::add_vote(&mut self.connection, &vote).await?;
                }
                else => break,
            }
        }

        let temp_path = NamedTempFile::new()
            .map_err(Error::io("creating named tempfile"))?
            .into_temp_path();
        export::vacuum_into(
            &self.connection,
            temp_path
                .to_str()
                .expect("named temp files generate unicode names"),
        )
        .await?;
        let data = block_in_place(|| {
            std::fs::read(&temp_path).map_err(Error::io("reading exported data from filesystem"))
        })?;

        Ok(data)
    }
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
    reaction_tx: async_channel::Sender<ReactionRequest>,
) {
    macro_rules! or_break {
        ($e:expr) => {
            if $e.is_err() {
                break;
            }
        };
    }

    while let Some(messages) = msg_rx.recv().await {
        for mut msg in messages {
            let reactions = std::mem::take(&mut msg.reactions);
            if let Ok(item) = (&msg).try_into() {
                for reaction in reactions {
                    or_break!(
                        reaction_tx
                            .send(ReactionRequest::new(&msg, reaction.reaction_type))
                            .await
                    );
                }

                or_break!(item_tx.send(item).await);
            }
        }
    }
}

/// Fetch the reaction details for a single `(message, reaction)` pair, dispatching votes and errors as appropriate.
async fn fetch_reaction_users(
    http: Arc<Http>,
    reaction_rx: async_channel::Receiver<ReactionRequest>,
    vote_tx: mpsc::Sender<Vote>,
    err_tx: mpsc::Sender<Error>,
) {
    macro_rules! dispatch_err {
        ($e:expr => $tx:expr) => {{
            match $e {
                Ok(ok) => ok,
                Err(e) => {
                    let _ = err_tx.send(e).await;
                    return;
                }
            }
        }};
    }

    const REACTION_USERS_LIMIT: u8 = 100;

    'receive: while let Ok(reaction_request) = reaction_rx.recv().await {
        let mut after = None;

        loop {
            let users = dispatch_err!(
                http.get_reaction_users(
                    reaction_request.channel_id,
                    reaction_request.message_id,
                    &reaction_request.reaction_type,
                    REACTION_USERS_LIMIT,
                    after
                ).await.map_err(Error::Http)
                => &err_tx
            );

            if users.is_empty() {
                continue 'receive;
            }
            after = users.last().map(|user| user.id.get());

            for user in users {
                if vote_tx
                    .send(Vote::new(
                        reaction_request.message_id,
                        user.id,
                        &reaction_request.reaction_type,
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }
    }
}

/// Collect votes and then reemit them on a separate channel when the prerequisite items have been persisted.
///
/// Votes cannot be persisted until the relevant item has been persisted. (They also cannot be persisted until
/// the relevant user id has been persisted, but user id persistence is a side effect of item persistence, so
/// we can ignore those.)
///
/// This function just keeps track of pending votes and items. Once an item has been persisted, the relevant
/// vote is released.
///
/// If any channel closes, this shuts down gracefully.
async fn hold_votes_for_relevant_item_persistence(
    mut persisted_items_rx: mpsc::Receiver<MessageId>,
    mut votes_rx: mpsc::Receiver<Vote>,
    persistable_votes_tx: mpsc::Sender<Vote>,
) {
    macro_rules! or_return {
        ($e:expr) => {
            if $e.is_err() {
                return;
            }
        };
    }

    let mut persisted_items = HashSet::new();
    let mut pending_votes = HashMap::<_, Vec<_>>::new();

    loop {
        select! {
            Some(persisted_item) = persisted_items_rx.recv() => {
                persisted_items.insert(persisted_item);
                if let Some(votes) = pending_votes.remove(&persisted_item) {
                    for vote in votes {
                        or_return!(persistable_votes_tx.send(vote).await);
                    }
                }
            }
            Some(vote) = votes_rx.recv() => {
                if persisted_items.contains(&vote.item_id) {
                    or_return!(persistable_votes_tx.send(vote).await);
                } else {
                    pending_votes.entry(vote.item_id).or_default().push(vote);
                }
            }
            else => break,
        }
    }
}
