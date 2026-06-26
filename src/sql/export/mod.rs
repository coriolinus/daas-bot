use rusqlite::{Connection, named_params};
use serenity::all::UserId;
use tokio::task::block_in_place;

use crate::{
    export::{ItemWithMetadata, Vote},
    sql::ToSqlInteger as _,
};

use super::Result;

/// Schema for the exported database.
///
/// As we never read or upgrade an existing database, but always generate a fresh one,
/// there's no need for any kind of migration framework; the schema file is always current.
///
/// Consuming applications can export the relevant schema from sqlite or get the right one
/// from Github.
const EXPORT_SCHEMA: &str = include_str!("schema.sql");

pub async fn apply_schema(connection: &Connection) -> Result<()> {
    block_in_place(|| connection.execute_batch(EXPORT_SCHEMA).map_err(Into::into))
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
async fn ensure_user(
    connection: &Connection,
    user_id: UserId,
    display_name: &str,
) -> Result<(Pk, bool)> {
    block_in_place(|| {
        // OR IGNORE means if the user id already existed, silently do nothing.
        let query = "INSERT OR IGNORE
            INTO users (id, display_name)
            VALUES (:user_id, :display_name)";

        let mut stmt = connection.prepare_cached(query)?;
        let rows = stmt.execute(named_params! {
            ":user_id": user_id.to_sql(),
            ":display_name": display_name,
        })?;

        Ok((user_id.to_sql(), rows != 0))
    })
}

/// Ensure a tag exists in the export database.
///
/// Returns the tag's primary key.
async fn ensure_tag(connection: &Connection, tag: &str) -> Result<Pk> {
    block_in_place(|| {
        let query = "INSERT INTO tags (description)
            VALUES (:description)
            ON CONFLICT DO NOTHING
            RETURNING id";

        let mut stmt = connection.prepare_cached(query)?;
        let id = stmt.query_one(
            named_params! {
                ":description": tag,
            },
            |row| row.get(0),
        )?;

        Ok(id)
    })
}

/// Ensure a tag association exists in the export database.
///
/// Returns true if the association was newly created.
async fn ensure_tag_association(connection: &Connection, item_id: Pk, tag_id: Pk) -> Result<bool> {
    block_in_place(|| {
        let query = "INSERT INTO tag_associations (item_id, tag_id)
            VALUES (:item_id, :tag_id)
            ON CONFLICT DO NOTHING";

        let mut stmt = connection.prepare_cached(query)?;
        let rows = stmt.execute(named_params! {
            ":item_id": item_id,
            ":tag_id": tag_id,
        })?;

        Ok(rows != 0)
    })
}

/// Add an item to the export database.
///
/// Returns the item's primary key.
pub async fn add_item(connection: &mut Connection, item: &ItemWithMetadata) -> Result<Pk> {
    // gives us rollback on error
    let transaction = connection.transaction()?;

    let (posted_by, _created) =
        ensure_user(&transaction, item.posted_by, &item.posted_by_display_name).await?;

    let item_id = block_in_place(|| -> Result<Pk> {
        let query = "INSERT INTO items (id, posted_by, title, description, created, edited)
            VALUES (:id, :posted_by, :title, :description, :created, :edited)
            RETURNING id";

        let mut stmt = transaction.prepare_cached(query)?;
        let id = stmt.query_one(
            named_params! {
                ":id": item.message_id.to_sql(),
                ":posted_by": posted_by,
                ":title": &item.item.title,
                ":description": &item.item.description,
                ":created": item.created_at.to_rfc3339().expect("infallible when using chrono-based timestamps"),
                ":edited": item.modified_at.map(|modified_at| modified_at.to_rfc3339().expect("infallible when using chrono-based timestamps")),
            },
            |row| row.get(0),
        )?;

        Ok(id)
    })?;

    for tag in &item.item.tags {
        let tag_id = ensure_tag(&transaction, tag).await?;
        ensure_tag_association(&transaction, item_id, tag_id).await?;
    }

    transaction.commit()?;
    Ok(item_id)
}

/// Ensure a category exists in the export database.
///
/// Returns the category's primary key.
async fn ensure_category(connection: &Connection, category: &str) -> Result<Pk> {
    block_in_place(|| {
        let query = "INSERT INTO categories (emoji)
            VALUES (:category)
            ON CONFLICT DO NOTHING
            RETURNING id";

        let mut stmt = connection.prepare_cached(query)?;
        let id = stmt.query_one(
            named_params! {
                ":category": category,
            },
            |row| row.get(0),
        )?;

        Ok(id)
    })
}

/// Add a vote to the export database.
///
/// This will error unless the appropriate user and items have already been added to the database.
/// Ensure those already exist before calling this!
pub async fn add_vote(connection: &mut Connection, vote: &Vote) -> Result<()> {
    // gives us rollback on error
    let transaction = connection.transaction()?;

    let category_id = ensure_category(&transaction, &vote.emoji).await?;

    block_in_place(|| -> Result<()> {
        let query = "INSERT INTO votes (item_id, user_id, category_id)
            VALUES (:item_id, :user_id, :category_id)";

        let mut stmt = transaction.prepare_cached(query)?;
        stmt.execute(named_params! {
            ":item_id": vote.item_id.to_sql(),
            ":user_id": vote.user_id.to_sql(),
            ":category_id": category_id,
        })?;

        Ok(())
    })?;

    transaction.commit()?;
    Ok(())
}

/// Export this database to the specified filename
pub async fn vacuum_into(connection: &Connection, path: &str) -> Result<()> {
    block_in_place(|| {
        let query = "VACUUM INTO :path";
        let mut stmt = connection.prepare_cached(query)?;
        stmt.execute(named_params! {":path": path})?;

        Ok(())
    })
}
