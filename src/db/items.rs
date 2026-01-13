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
        self.add_item_count(chat_id, text).await?;
        Ok(())
    }

    pub async fn add_item_count(&self, chat_id: ChatId, text: &str) -> Result<u64> {
        tracing::trace!(chat_id = chat_id.0, text = %text, "Adding item");
        let result = sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
            .bind(chat_id.0)
            .bind(text)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn add_items(&self, chat_id: ChatId, items: &[String]) -> Result<()> {
        self.add_items_count(chat_id, items).await?;
        Ok(())
    }

    pub async fn add_items_count(&self, chat_id: ChatId, items: &[String]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }

        tracing::trace!(chat_id = chat_id.0, count = items.len(), "Adding items");
        let mut builder =
            sqlx::QueryBuilder::<sqlx::Sqlite>::new("INSERT INTO items (chat_id, text) ");
        builder.push_values(items, |mut row, text| {
            row.push_bind(chat_id.0).push_bind(text);
        });

        let result = builder.build().execute(self.pool()).await?;
        Ok(result.rows_affected())
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
        self.toggle_item_count(chat_id, id).await?;
        Ok(())
    }

    pub async fn toggle_item_count(&self, chat_id: ChatId, id: i64) -> Result<u64> {
        tracing::trace!(chat_id = chat_id.0, item_id = id, "Toggling item");
        let result = sqlx::query("UPDATE items SET done = NOT done WHERE id = ? AND chat_id = ?")
            .bind(id)
            .bind(chat_id.0)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_item(&self, chat_id: ChatId, id: i64) -> Result<()> {
        self.delete_item_count(chat_id, id).await?;
        Ok(())
    }

    pub async fn delete_item_count(&self, chat_id: ChatId, id: i64) -> Result<u64> {
        tracing::trace!(chat_id = chat_id.0, item_id = id, "Deleting item");
        let result = sqlx::query("DELETE FROM items WHERE id = ? AND chat_id = ?")
            .bind(id)
            .bind(chat_id.0)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_all_items(&self, chat_id: ChatId) -> Result<()> {
        self.delete_all_items_count(chat_id).await?;
        Ok(())
    }

    pub async fn delete_all_items_count(&self, chat_id: ChatId) -> Result<u64> {
        tracing::debug!(chat_id = chat_id.0, "Deleting all items");
        let result = sqlx::query("DELETE FROM items WHERE chat_id = ?")
            .bind(chat_id.0)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_items(&self, chat_id: ChatId, ids: &[i64]) -> Result<()> {
        self.delete_items_count(chat_id, ids).await?;
        Ok(())
    }

    pub async fn delete_items_count(&self, chat_id: ChatId, ids: &[i64]) -> Result<u64> {
        tracing::trace!(chat_id = chat_id.0, ?ids, "Deleting multiple items");
        if ids.is_empty() {
            return Ok(0);
        }

        let mut builder =
            sqlx::QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM items WHERE chat_id = ");
        builder.push_bind(chat_id.0);
        builder.push(" AND id IN (");
        {
            let mut separated = builder.separated(", ");
            for id in ids {
                separated.push_bind(id);
            }
        }
        builder.push(")");

        let result = builder.build().execute(self.pool()).await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;

    #[tokio::test]
    async fn add_items_inserts_multiple() {
        let db = init_test_db().await;
        let chat = ChatId(1);
        let items = vec!["Apple".to_string(), "Banana".to_string()];

        let added = db.add_items_count(chat, &items).await.unwrap();
        assert_eq!(added, 2);

        let stored = db.list_items(chat).await.unwrap();
        let texts: Vec<_> = stored.into_iter().map(|item| item.text).collect();
        assert_eq!(texts, vec!["Apple".to_string(), "Banana".to_string()]);
    }

    #[tokio::test]
    async fn add_items_handles_empty() {
        let db = init_test_db().await;
        let chat = ChatId(1);
        let items: Vec<String> = Vec::new();

        let added = db.add_items_count(chat, &items).await.unwrap();
        assert_eq!(added, 0);

        let stored = db.list_items(chat).await.unwrap();
        assert!(stored.is_empty());
    }
}
