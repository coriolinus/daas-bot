use rusqlite::{Connection, named_params};
use serenity::all::UserId;
use tokio::{sync::OwnedMutexGuard, task::spawn_blocking};

use crate::{
    export::{ItemWithMetadata, Vote},
    sql::ToSqlInteger as _,
};

use super::{Error, Result};

/// Schema for the exported database.
///
/// As we never read or upgrade an existing database, but always generate a fresh one,
/// there's no need for any kind of migration framework; the schema file is always current.
///
/// Consuming applications can export the relevant schema from sqlite or get the right one
/// from Github.
const EXPORT_SCHEMA: &str = include_str!("schema.sql");

pub async fn apply_schema(connection: OwnedMutexGuard<Connection>) -> Result<()> {
    spawn_blocking(move || {
        connection
            .execute_batch(EXPORT_SCHEMA)
            .map_err(Error::sql("applying schema to export db"))
    })
    .await
    .map_err(Into::into)
    .flatten()
}

/// An `INTEGER PRIMARY KEY` identifying a row in the database.
///
/// Because Sqlite isn't the best at unsigned 64-bit integers, this is modeled
/// as a signed integer. In realistic terms this shouldn't ever matter; just use caution
/// when attempting to convert back into a Discord Snowflake or similar.
/// The binary representation is correct, but the value may not be.
pub type Pk = i64;

/// Ensure a user exists in the export database.
///
/// If the user already exists in the database, we assume that its name is more current
/// than the name we've found, and abort early.
///
/// Returns the user's primary key and whether or not the user was created.
fn ensure_user(connection: &Connection, user_id: UserId, display_name: &str) -> Result<(Pk, bool)> {
    // OR IGNORE means if the user id already existed, silently do nothing.
    let query = "INSERT OR IGNORE
            INTO users (id, display_name)
            VALUES (:user_id, :display_name)";

    let mut stmt = connection
        .prepare_cached(query)
        .map_err(Error::sql("preparing statement to ensure user"))?;
    let rows = stmt
        .execute(named_params! {
            ":user_id": user_id.to_sql(),
            ":display_name": display_name,
        })
        .map_err(Error::sql("executing statement to ensure user"))?;

    Ok((user_id.to_sql(), rows != 0))
}

/// Ensure a tag exists in the export database.
///
/// Returns the tag's primary key.
fn ensure_tag(connection: &Connection, tag: &str) -> Result<Pk> {
    // note the on conflict thing; we do need to touch the row to return the id
    let query = "INSERT INTO tags (description)
            VALUES (:description)
            ON CONFLICT (description) DO UPDATE SET description = excluded.description
            RETURNING id";

    let mut stmt = connection
        .prepare_cached(query)
        .map_err(Error::sql("preparing statement to ensure tag"))?;
    let id = stmt
        .query_one(
            named_params! {
                ":description": tag,
            },
            |row| row.get(0),
        )
        .map_err(Error::sql("executing statement to ensure tag"))?;

    Ok(id)
}

/// Ensure a tag association exists in the export database.
///
/// Returns true if the association was newly created.
fn ensure_tag_association(connection: &Connection, item_id: Pk, tag_id: Pk) -> Result<bool> {
    let query = "INSERT INTO tag_associations (item_id, tag_id)
            VALUES (:item_id, :tag_id)
            ON CONFLICT DO NOTHING";

    let mut stmt = connection
        .prepare_cached(query)
        .map_err(Error::sql("preparing statement to ensure tag association"))?;
    let rows = stmt
        .execute(named_params! {
            ":item_id": item_id,
            ":tag_id": tag_id,
        })
        .map_err(Error::sql("executing statement to ensure tag association"))?;

    Ok(rows != 0)
}

/// Add an item to the export database.
///
/// Returns the item's primary key.
pub async fn add_item(
    mut connection: OwnedMutexGuard<Connection>,
    item: ItemWithMetadata,
) -> Result<Pk> {
    spawn_blocking(move || -> Result<Pk> {

    // gives us rollback on error
    let transaction = connection.transaction().map_err(Error::sql("creating transaction to add item"))?;

    let (posted_by, _created) =
        ensure_user(&transaction, item.posted_by, &item.posted_by_display_name)?;

        let query = "INSERT INTO items (id, posted_by, title, description, created, edited)
            VALUES (:id, :posted_by, :title, :description, :created, :edited)
            RETURNING id";

        let item_id = {
            let mut stmt = transaction.prepare_cached(query).map_err(Error::sql("preparing statement to add item"))?;
            stmt.query_one(
                named_params! {
                    ":id": item.message_id.to_sql(),
                    ":posted_by": posted_by,
                    ":title": &item.item.title,
                    ":description": &item.item.description,
                    ":created": item.created_at.to_rfc3339().expect("infallible when using chrono-based timestamps"),
                    ":edited": item.modified_at.map(|modified_at| modified_at.to_rfc3339().expect("infallible when using chrono-based timestamps")),
                },
                |row| row.get(0),
            ).map_err(Error::sql("executing statement to add item"))?
        };

        for tag in &item.item.tags {
            let tag_id = ensure_tag(&transaction, tag)?;
            ensure_tag_association(&transaction, item_id, tag_id)?;
        }

        transaction.commit().map_err(Error::sql("commiting transaction to add item"))?;

        Ok(item_id)
    }).await.map_err(Into::into).flatten()
}

/// Ensure a category exists in the export database.
///
/// Returns the category's primary key.
fn ensure_category(connection: &Connection, category: &str) -> Result<Pk> {
    // note the on conflict thing; we do need to touch the row to return the id
    let query = "INSERT INTO categories (emoji)
            VALUES (:category)
            ON CONFLICT (emoji) DO UPDATE SET emoji = excluded.emoji
            RETURNING id";

    let mut stmt = connection
        .prepare_cached(query)
        .map_err(Error::sql("preparing statement to ensure category"))?;
    let id = stmt
        .query_one(
            named_params! {
                ":category": category,
            },
            |row| row.get(0),
        )
        .map_err(Error::sql("executing statement to ensure category"))?;

    Ok(id)
}

/// Add a vote to the export database.
///
/// This will error unless the appropriate user and items have already been added to the database.
/// Ensure those already exist before calling this!
pub async fn add_vote(mut connection: OwnedMutexGuard<Connection>, vote: Vote) -> Result<()> {
    spawn_blocking(move || -> Result<()> {
        // gives us rollback on error
        let transaction = connection
            .transaction()
            .map_err(Error::sql("creating transaction to add vote"))?;

        let user_id = ensure_user(&transaction, vote.user_id, &vote.user_display_name)?.0;
        let category_id = ensure_category(&transaction, &vote.emoji)?;

        let query = "INSERT INTO votes (item_id, user_id, category_id)
            VALUES (:item_id, :user_id, :category_id)";

        {
            let mut stmt = transaction
                .prepare_cached(query)
                .map_err(Error::sql("preparing statement to add vote"))?;
            stmt.execute(named_params! {
                ":item_id": vote.item_id.to_sql(),
                ":user_id": user_id,
                ":category_id": category_id,
            })
            .map_err(Error::sql("executing statement to add vote"))?;
        }

        transaction
            .commit()
            .map_err(Error::sql("commiting transaction to add vote"))?;
        Ok(())
    })
    .await
    .map_err(Into::into)
    .flatten()
}

/// Export this database to the specified filename
pub async fn vacuum_into(
    connection: OwnedMutexGuard<Connection>,
    path: impl Into<String>,
) -> Result<()> {
    let path = path.into();
    spawn_blocking(move || {
        let query = "VACUUM INTO :path";
        let mut stmt = connection
            .prepare_cached(query)
            .map_err(Error::sql("preparing statement to export database"))?;
        stmt.execute(named_params! {":path": path})
            .map_err(Error::sql("executing statement to export database"))?;

        Ok(())
    })
    .await
    .map_err(Into::into)
    .flatten()
}
