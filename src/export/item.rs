use std::str::FromStr;

use serenity::all::{Message, MessageId, Timestamp, UserId};

#[derive(Debug, thiserror::Error)]
#[error("failed to parse input as Item")]
pub(crate) struct ParseError;

pub(crate) struct Item {
    pub(crate) title: String,
    pub(crate) tags: Vec<String>,
    pub(crate) description: String,
}

impl FromStr for Item {
    type Err = ParseError;

    fn from_str(mut input: &str) -> Result<Self, ParseError> {
        input = input.trim_start();
        if input.starts_with("**") {
            input = &input[2..];
        } else {
            return Err(ParseError);
        }

        // find matching close-bold and therefore the title
        let Some(title_terminal_idx) = input.find("**") else {
            return Err(ParseError);
        };
        let (title, rest) = input.split_at(title_terminal_idx);
        debug_assert!(rest.starts_with("**"));
        input = &rest[2..];

        let title = title.to_owned();

        // parse tags
        let mut tags = Vec::new();
        loop {
            input = input.trim_start();
            if !input.starts_with('_') {
                break;
            }
            input = &input[1..];

            let Some(tag_terminal_idx) = input.find('_') else {
                return Err(ParseError);
            };
            let (tag, rest) = input.split_at(tag_terminal_idx);
            tags.push(tag.to_owned());
            debug_assert!(rest.starts_with('_'));
            input = &rest[1..];
        }

        // description is everything else
        let description = input.trim().to_owned();

        Ok(Self {
            title,
            tags,
            description,
        })
    }
}

pub(crate) struct ItemWithMetadata {
    pub(crate) item: Item,
    pub(crate) message_id: MessageId,
    pub(crate) posted_by: UserId,
    pub(crate) posted_by_display_name: String,
    pub(crate) created_at: Timestamp,
    pub(crate) modified_at: Option<Timestamp>,
}

impl TryFrom<Message> for ItemWithMetadata {
    type Error = ParseError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        let item = msg.content.parse()?;

        let message_id = msg.id;
        let posted_by = msg.author.id;
        let posted_by_display_name = msg.author.display_name().to_owned();
        let created_at = msg.timestamp;
        let modified_at = msg.edited_timestamp;

        Ok(Self {
            item,
            message_id,
            posted_by,
            posted_by_display_name,
            created_at,
            modified_at,
        })
    }
}
