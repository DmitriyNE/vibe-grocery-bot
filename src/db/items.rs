use super::Database;
use anyhow::Result;
use teloxide::types::ChatId;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct Item {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

impl Database {
    pub async fn add_item(&self, chat_id: ChatId, text: &str) -> Result<()> {
        tracing::trace!(chat_id = chat_id.0, text = %text, "Adding item");
        sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
            .bind(chat_id.0)
            .bind(text)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_items(&self, chat_id: ChatId) -> Result<Vec<Item>> {
        tracing::trace!(chat_id = chat_id.0, "Listing items");
        sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
            .bind(chat_id.0)
            .fetch_all(self.pool())
            .await
            .map_err(Into::into)
    }

    pub async fn toggle_item(&self, id: i64) -> Result<()> {
        tracing::trace!(item_id = id, "Toggling item");
        sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_item(&self, id: i64) -> Result<()> {
        tracing::trace!(item_id = id, "Deleting item");
        sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_all_items(&self, chat_id: ChatId) -> Result<()> {
        tracing::debug!(chat_id = chat_id.0, "Deleting all items");
        sqlx::query("DELETE FROM items WHERE chat_id = ?")
            .bind(chat_id.0)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_items(&self, ids: &[i64]) -> Result<()> {
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
        builder.build().execute(self.pool()).await?;
        Ok(())
    }
}
