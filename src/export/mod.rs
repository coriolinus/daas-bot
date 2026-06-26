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

use log::{debug, trace, warn};
use rusqlite::Connection;
use serenity::all::{
    ChannelId, CommandInteraction, Http, Message, MessageId, MessagePagination, ReactionType,
};
use tempfile::NamedTempFile;
use tokio::{
    select, spawn,
    sync::{Mutex, mpsc},
    task::spawn_blocking,
};

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

/// Break out of an event loop in the event of an error.
macro_rules! or_break {
    ($e:expr $(=> $label:lifetime)? $(; $log:literal)?) => {
        if $e.is_err() {
            $(warn!($log);)?
            break $($label)?;
        }
    };
}

/// Dispatch an error on the specified channel and break if the expression results in one.
macro_rules! dispatch_err {
    ($e:expr => $tx:expr $(; $log:literal)?) => {{
        match $e {
            Ok(ok) => ok,
            Err(e) => {
                $(warn!($log);)?
                // we don't care if the send fails in this case; we're breaking regardless
                let _ = $tx.send(e).await;
                break;
            }
        }
    }};
}

/// Container type which holds fundamental data about the command which launched this export,
/// a handle to the Http client, and the Sqlite connection.
///
/// This struct acts as a receiver onto which other
#[derive(Debug)]
pub struct Exporter {
    interaction: CommandInteraction,
    http: Arc<Http>,
    connection: Arc<Mutex<Connection>>,
}

impl Exporter {
    pub async fn new(interaction: CommandInteraction, http: Arc<Http>) -> Result<Self> {
        let connection = Arc::new(Mutex::new(Connection::open_in_memory().map_err(
            crate::sql::Error::sql("opening database for export in memory"),
        )?));
        export::apply_schema(connection.clone().lock_owned().await).await?;

        Ok(Self {
            interaction,
            http,
            connection,
        })
    }

    /// Drive this export to completion.
    pub async fn drive(self) -> Result<Vec<u8>> {
        const HIGH_BUFFER_SIZE: usize = 128;
        const REACTION_PROCESSING_TASKS: usize = 8;

        debug!("exporter: drive: start");

        // this scope ensures we don't accidentally keep around any channel ends whose closing is important to the system's signaling
        let (mut error_rx, mut item_rx, mut persistable_votes_rx, mut persisted_items_tx) = {
            let (message_pages_tx, message_pages_rx) = mpsc::channel(2); // provide backpressure
            let (error_tx, error_rx) = mpsc::channel(1);
            let (item_tx, item_rx) = mpsc::channel(HIGH_BUFFER_SIZE);
            let (reaction_tx, reaction_rx) = async_channel::bounded(HIGH_BUFFER_SIZE);
            let (votes_tx, votes_rx) = mpsc::channel(HIGH_BUFFER_SIZE);
            let (persisted_items_tx, persisted_items_rx) = mpsc::channel(2);
            let (persistable_votes_tx, persistable_votes_rx) = mpsc::channel(HIGH_BUFFER_SIZE);

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

            (error_rx, item_rx, persistable_votes_rx, persisted_items_tx)
        };

        debug!("exporter: drive: beginning event loop");

        loop {
            select! {
                Some(err) = error_rx.recv() => {
                    debug!("exporter: drive: rx error ({err}), aborting");
                    return Err(err)},
                maybe_item = item_rx.recv(), if !persisted_items_tx.is_closed() => {
                    let Some(item) = maybe_item else {
                        // When the incoming items channel finishes, we need to close the outbound
                        // persisted_items channel so that `hold_votes_for_relevant_item_persistence`
                        // can correctly judge when to shut itself down.
                        // Challenge: we don't have a `.close` method on the transmitter.
                        // Solution: replace it with a dummy one whose receiver immediately drops.
                        // Note that we don't keep the rx side of things, so the new receiver immediately
                        // drops, so the guard on this select branch should prevent a fast loop.
                        let (dummy_tx, _dummy_rx) = mpsc::channel(1); // mpsc bounded channel requires buffer > 0
                        persisted_items_tx = dummy_tx;
                        continue;
                    };
                    debug!("exporter: drive: rx item, persisting");
                    let message_id = item.message_id;
                    export::add_item(self.connection.clone().lock_owned().await, item).await?;
                    persisted_items_tx.send(message_id).await.map_err(Error::send_failed("persisted_items"))?;
                }
                Some(vote) = persistable_votes_rx.recv() => {
                    debug!("exporter: drive: rx vote, persisting");
                    export::add_vote(self.connection.clone().lock_owned().await, vote).await?;
                }
                else => {
                    debug!("exporter: drive: all input channels closed; breaking event loop");
                    break;
                },
            }
        }

        debug!("exporter: drive: beginning export of in-memory db to disk");
        let temp_path = NamedTempFile::new()
            .map_err(Error::io("creating named tempfile"))?
            .into_temp_path();
        export::vacuum_into(
            self.connection.clone().lock_owned().await,
            temp_path
                .to_str()
                .expect("named temp files generate unicode names"),
        )
        .await?;
        let data = spawn_blocking(move || {
            std::fs::read(&temp_path).map_err(Error::io("reading exported data from filesystem"))
        })
        .await
        .map_err(Error::ReadFilesystemJoinFailure)
        .flatten()?;

        debug!("exporter: drive: successfully read export, finished");
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
    debug!("exporter: fetch_message_pages: starting task");
    let mut before = None;

    loop {
        // https://docs.discord.com/developers/resources/message#get-channel-messages:
        // > limit?	integer	Max number of messages to return (1-100)
        const GET_MESSAGES_LIMIT: Option<u8> = Some(100);

        debug!("exporter: fetch_message_pages: fetching messages before msg id \"{before:?}\"");

        let messages = dispatch_err!(
            http
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
            => err_tx;
            "exporter: fetch_message_pages: failed to fetch messages, aborting"
        );

        if messages.is_empty() {
            // we've exhausted the messages in the channel
            debug!("exporter: fetch_message_pages: no messages received, ending task");
            break;
        }
        or_break!(msg_tx.send(messages).await; "exporter: fetch_message_pages: msg_tx send failed, aborting");
    }

    debug!("exporter: fetch_message_pages: ending task");
}

/// Process a batch of messages: parse them into `ItemWithMetadata` instances and send each to the relevant sender
async fn process_messages(
    mut msg_rx: mpsc::Receiver<Vec<Message>>,
    item_tx: mpsc::Sender<ItemWithMetadata>,
    reaction_tx: async_channel::Sender<ReactionRequest>,
) {
    debug!("exporter: process_messages: starting task");
    while let Some(messages) = msg_rx.recv().await {
        debug!(
            "exporter: process_messages: received {} messages",
            messages.len()
        );

        for mut msg in messages {
            let reactions = std::mem::take(&mut msg.reactions);
            if let Ok(item) = (&msg).try_into() {
                trace!(
                    "exporter: process_messages: message parsed as item, dispatching reaction request"
                );
                for reaction in reactions {
                    or_break!(
                        reaction_tx
                            .send(ReactionRequest::new(&msg, reaction.reaction_type))
                            .await;
                        "exporter: process_messages: reaction_tx send failed, aborting"
                    );
                }

                or_break!(item_tx.send(item).await; "exporter: process_messages: item_tx send failed, aborting");
            } else {
                trace!("exporter: process_messages: message did not parse as item");
            }
        }
    }
    debug!("exporter: process_messages: ending task");
}

/// Fetch the reaction details for a single `(message, reaction)` pair, dispatching votes and errors as appropriate.
async fn fetch_reaction_users(
    http: Arc<Http>,
    reaction_rx: async_channel::Receiver<ReactionRequest>,
    vote_tx: mpsc::Sender<Vote>,
    err_tx: mpsc::Sender<Error>,
) {
    debug!("exporter: fetch_reaction_users: starting task");

    const REACTION_USERS_LIMIT: u8 = 100;

    'receive: while let Ok(reaction_request) = reaction_rx.recv().await {
        let mut after = None;

        loop {
            debug!(
                "exporter: fetch_reaction_users: getting reaction users for msg {} after user id {after:?}",
                reaction_request.message_id,
            );

            let users = dispatch_err!(
                http.get_reaction_users(
                    reaction_request.channel_id,
                    reaction_request.message_id,
                    &reaction_request.reaction_type,
                    REACTION_USERS_LIMIT,
                    after
                ).await.map_err(Error::Http)
                => err_tx;
                "exporter: fetch_reaction_users: failed to fetch reaction users, aborting"
            );

            if users.is_empty() {
                debug!(
                    "exporter: fetch_reaction_users: received no users, user collection complete"
                );
                continue 'receive;
            }
            after = users.last().map(|user| user.id.get());

            for user in users {
                or_break!(
                    vote_tx
                    .send(Vote::new(
                        reaction_request.message_id,
                        user.id,
                        user.display_name().to_owned(),
                        &reaction_request.reaction_type,
                    ))
                    .await => 'receive;
                    "exporter: fetch_reaction_users: vote_tx send failed, aborting"
                );
            }
        }
    }

    debug!("exporter: fetch_reaction_users: ending task");
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
    debug!("exporter: hold_votes_for_relevant_item_persistence: starting task");

    let mut persisted_items = HashSet::new();
    let mut pending_votes = HashMap::<_, Vec<_>>::new();

    'select: loop {
        select! {
            Some(persisted_item) = persisted_items_rx.recv() => {
                persisted_items.insert(persisted_item);
                if let Some(votes) = pending_votes.remove(&persisted_item) {
                    debug!("exporter: hold_votes_for_relevant_item_persistence: rx persisted item, releasing {} pending votes", votes.len());
                    for vote in votes {
                        or_break!(
                            persistable_votes_tx.send(vote).await => 'select;
                            "exporter: hold_votes_for_relevant_item_persistence: send cached persistable vote failed, aborting"
                        );
                    }
                } else {
                    debug!("exporter: hold_votes_for_relevant_item_persistence: rx persisted item, no pending votes yet");
                }
            }
            Some(vote) = votes_rx.recv() => {
                if persisted_items.contains(&vote.item_id) {
                    debug!("exporter: hold_votes_for_relevant_item_persistence: rx vote, item already persisted, forwarding");
                    or_break!(
                        persistable_votes_tx.send(vote).await;
                        "exporter: hold_votes_for_relevant_item_persistence: forwarding persistable vote failed, aborting"
                    );
                } else {
                    debug!("exporter: hold_votes_for_relevant_item_persistence: rx vote, item not yet persisted, caching");
                    pending_votes.entry(vote.item_id).or_default().push(vote);
                }
            }
            else => {
                debug!("exporter: hold_votes_for_relevant_item_persistence: all input channels closed; breaking event loop");
                break;
            },
        }
    }

    debug!("exporter: hold_votes_for_relevant_item_persistence: ending task");
}
