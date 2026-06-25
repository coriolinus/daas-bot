use log::info;
use serenity::all::{CommandInteraction, CreateInteractionResponseMessage};

use crate::server::{Message, Result};

const HELP_MESSAGE: &str = "# **Discord As A Spreadsheet Bot** Help

## Commands

The DAAS Bot exposes one slash command: `/daas`. It exposes these subcommands:

- `/daas help`: Emit this help message.
- `/daas enable`: Enable export for the current channel. Callable by admins only. Enabling is required prior to export to prevent rate limit exhaustion attacks.
- `/daas disable`: Disable export for the current channel. Callable by admins only.
- `/daas export`: Look through the messages of this channel for those matching the item format. Collect those messages and their reactions, and export a database with that data. Callable by any user, but only after the channel has been enabled by an admin.
- `/daas cleanup`: Look through the messages of this channel and delete all but the most recent export which this bot has produced. Callable by any user, but only after the channel has been enabled by an admin.

## Item Format

On export, DAAS Bot looks through the channel's entire history for messages in this format:

```
**<Title>** (_<tag>_ (_<tag>_ ...)) <Description>
```

Strictly, any message which begins with a `**bold**` string and contains non-whitespace text after the title is a parseable item.

Any number of `_italic_` tags can optionally be applied to an item; the only limitations are that they are separated by whitespace, and each immediately follows either the title or a preceding tag.

## Output

The output is a sqlite database containing the following tables:

- `items (id, title, description)`
- `tags (id, description)`
- `tag_associations (item_id, tag_id)`
- `users (id, display_name)`
- `categories (id, emoji)`
- `votes (id, item_id, user_id, category_id)`

Nothing is nullable. Ids are numeric. Everything else is text. (Timestamps are [encoded in ISO-8601](https://sqlite.org/lang_datefunc.html#tmval).)
";

/// Immediately return a help message giving an overview of what the commands are and what each does.
pub async fn help(_interation: CommandInteraction) -> Result<Message> {
    info!("handling help interaction");
    Ok(CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .content(HELP_MESSAGE)
        .into())
}
