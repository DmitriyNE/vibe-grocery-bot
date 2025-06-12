use super::Database;
use crate::db::types::{ChatKey, ItemId};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub text: String,
    pub done: bool,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: i64,
    text: String,
    done: bool,
}

impl Database {
    pub async fn add_item(&self, chat_id: ChatKey, text: &str) -> Result<()> {
        tracing::trace!(chat_id = chat_id.0, text = %text, "Adding item");
        sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
            .bind::<i64>(chat_id.into())
            .bind(text)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_items(&self, chat_id: ChatKey) -> Result<Vec<Item>> {
        tracing::trace!(chat_id = chat_id.0, "Listing items");
        let rows: Vec<ItemRow> =
            sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
                .bind::<i64>(chat_id.into())
                .fetch_all(self.pool())
                .await?;
        Ok(rows
            .into_iter()
            .map(|r| Item {
                id: ItemId(r.id),
                text: r.text,
                done: r.done,
            })
            .collect())
    }

    pub async fn toggle_item(&self, chat_id: ChatKey, id: ItemId) -> Result<()> {
        let id_val: i64 = id.into();
        tracing::trace!(chat_id = chat_id.0, item_id = id_val, "Toggling item");
        sqlx::query("UPDATE items SET done = NOT done WHERE id = ? AND chat_id = ?")
            .bind(id_val)
            .bind::<i64>(chat_id.into())
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_item(&self, chat_id: ChatKey, id: ItemId) -> Result<()> {
        let id_val: i64 = id.into();
        tracing::trace!(chat_id = chat_id.0, item_id = id_val, "Deleting item");
        sqlx::query("DELETE FROM items WHERE id = ? AND chat_id = ?")
            .bind(id_val)
            .bind::<i64>(chat_id.into())
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_all_items(&self, chat_id: ChatKey) -> Result<()> {
        tracing::debug!(chat_id = chat_id.0, "Deleting all items");
        sqlx::query("DELETE FROM items WHERE chat_id = ?")
            .bind::<i64>(chat_id.into())
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_items(&self, chat_id: ChatKey, ids: &[ItemId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let id_values: Vec<i64> = ids.iter().copied().map(Into::into).collect();
        tracing::trace!(chat_id = chat_id.0, ?id_values, "Deleting multiple items");

        let mut builder =
            sqlx::QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM items WHERE chat_id = ");
        builder.push_bind::<i64>(chat_id.into());
        builder.push(" AND id IN (");
        {
            let mut separated = builder.separated(", ");
            for id in &id_values {
                separated.push_bind(id);
            }
        }
        builder.push(")");

        builder.build().execute(self.pool()).await?;
        Ok(())
    }
}
