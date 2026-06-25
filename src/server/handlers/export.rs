use std::{fmt::Write as _, sync::Arc, thread};

use either::Either;
use jiff::Timestamp;
use log::{error, info, warn};
use serenity::all::{
    CommandInteraction, CreateAttachment, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, Http,
};
use tokio::{runtime, task::LocalSet};

use crate::{
    export::Exporter,
    server::{AppState, Defer, Error, Message, Result},
    sql::channel_is_enabled,
};

/// Export the data gathered in this channel to an sqlite file.
///
/// See the README for parseable message format and data format.
///
/// 1. verify that this is an enabled channel.
/// 2. kick off an async task to export the data into sql (see readme)
/// 3. while that's running, respond with this defer message
pub async fn export(
    interaction: CommandInteraction,
    app_state: &AppState,
) -> Result<Either<Message, Defer>> {
    info!("handling export interaction");
    let guild = interaction
        .guild_id
        .ok_or(Error::MalformedInput("no guild id"))?;

    let connection = app_state.local_db.lock().await;

    if !channel_is_enabled(&connection, guild, interaction.channel_id).await? {
        return Ok(Either::Left(Message::from(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("permission denied"),
        )));
    }

    // we need special handling for this future because it has a Sqlite connection,
    // which is `!Send`, so we need to ensure that this task never moves between threads.
    let http = app_state.http.clone();
    thread::spawn(move || {
        let Ok(rt) = runtime::Builder::new_current_thread().enable_all().build() else {
            error!("failed to create a current-thread runtime for the local set");
            return;
        };
        rt.block_on(async {
            LocalSet::new()
                .run_until(gather_export_and_update_response(interaction, http))
                .await;
        });
    });

    Ok(Either::Right(Defer::from(
        CreateInteractionResponseMessage::new(),
    )))
}

async fn gather_export_and_update_response(interaction: CommandInteraction, http: Arc<Http>) {
    info!("starting export task");

    let token = interaction.token.clone();
    let channel_name = interaction
        .channel
        .as_ref()
        .and_then(|channel| channel.name.as_deref())
        .unwrap_or("this channel")
        .to_owned();
    let now = Timestamp::now();

    // note that when desugared, the future in this async block is sufficient to capture
    // the return value, so the `?` operator stops at the block scope, not the enclosing function scope.
    let (followup, data) = async {
        let exporter = Exporter::new(interaction, http.clone()).await?;
        let data = exporter.drive().await?;

        let duration = Timestamp::now() - now;

        let content = format!("Exported {channel_name} at {now} in {duration}");
        info!("successfully completed export job");

        Ok((
            CreateInteractionResponseFollowup::new().content(content),
            Some(data),
        ))
    }
    .await
    .unwrap_or_else(|err: Error| {
        let mut content = "daas-bot encountered a problem while performing export:\n\n".to_owned();

        warn!("failed to complete export job due to {err}");

        let mut err: Option<&dyn std::error::Error> = Some(&err);
        while let Some(top_level) = err {
            let _ = writeln!(&mut content, "{top_level}");
            err = top_level.source();
        }

        (
            CreateInteractionResponseFollowup::new().content(content),
            None,
        )
    });

    let files = data
        .map(|data| {
            CreateAttachment::bytes(data, "export.sqlite").description(format!(
                "The items and votes tabulated in {channel_name}, as calculated at {now} by Daas Bot"
            ))
        })
        .into_iter()
        .collect();

    let _ = http.create_followup_message(&token, &followup, files).await;
}
