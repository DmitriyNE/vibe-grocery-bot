use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::types::ChatId;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct Item {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

pub async fn add_item(db: &Pool<Sqlite>, chat_id: ChatId, text: &str) -> Result<()> {
    tracing::trace!(chat_id = chat_id.0, text = %text, "Adding item");
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat_id.0)
        .bind(text)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<Item>> {
    tracing::trace!(chat_id = chat_id.0, "Listing items");
    sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
        .bind(chat_id.0)
        .fetch_all(db)
        .await
        .map_err(Into::into)
}

pub async fn toggle_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    tracing::trace!(item_id = id, "Toggling item");
    sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    tracing::trace!(item_id = id, "Deleting item");
    sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_all_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Deleting all items");
    sqlx::query("DELETE FROM items WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_items(db: &Pool<Sqlite>, ids: &[i64]) -> Result<()> {
    tracing::trace!(?ids, "Deleting multiple items");
    if ids.is_empty() {
        return Ok(());
    }

    let mut builder = sqlx::QueryBuilder::new("DELETE FROM items WHERE id IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");
    builder.build().execute(db).await?;
    Ok(())
}
