use serenity::all::{MessageId, Reaction, ReactionType, UserId};

fn emoji_of(reaction: &ReactionType) -> String {
    match reaction {
        ReactionType::Unicode(emoji) => emoji.to_owned(),
        ReactionType::Custom { id, name, .. } => name.clone().unwrap_or_else(|| id.to_string()),
        _ => reaction.as_data(),
    }
}

pub(crate) struct Vote {
    pub(crate) item_id: MessageId,
    pub(crate) user_id: UserId,
    pub(crate) emoji: String,
}

impl Vote {
    pub(crate) fn new(message_id: MessageId, user_id: UserId, reaction: &ReactionType) -> Self {
        Self {
            user_id,
            item_id: message_id,
            emoji: emoji_of(reaction),
        }
    }
}
