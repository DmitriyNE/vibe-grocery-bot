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

    pub async fn toggle_item(&self, chat_id: ChatId, id: i64) -> Result<()> {
        tracing::trace!(chat_id = chat_id.0, item_id = id, "Toggling item");
        sqlx::query("UPDATE items SET done = NOT done WHERE id = ? AND chat_id = ?")
            .bind(id)
            .bind(chat_id.0)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_item(&self, chat_id: ChatId, id: i64) -> Result<()> {
        tracing::trace!(chat_id = chat_id.0, item_id = id, "Deleting item");
        sqlx::query("DELETE FROM items WHERE id = ? AND chat_id = ?")
            .bind(id)
            .bind(chat_id.0)
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

    pub async fn delete_items(&self, chat_id: ChatId, ids: &[i64]) -> Result<()> {
        tracing::trace!(chat_id = chat_id.0, ?ids, "Deleting multiple items");
        if ids.is_empty() {
            return Ok(());
        }

        let mut sql = String::from("DELETE FROM items WHERE chat_id = ? AND id IN (");
        sql.push_str(&vec!["?"; ids.len()].join(", "));
        sql.push(')');
        let mut query = sqlx::query(&sql).bind(chat_id.0);
        for id in ids {
            query = query.bind(id);
        }
        query.execute(self.pool()).await?;
        Ok(())
    }
}
