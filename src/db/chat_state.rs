use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::types::{ChatId, MessageId};

#[derive(sqlx::FromRow)]
struct ChatState {
    last_list_message_id: i32,
}

pub async fn get_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Option<i32>> {
    tracing::trace!(chat_id = chat_id.0, "Fetching last list message id");
    let result = sqlx::query_as::<_, ChatState>(
        "SELECT last_list_message_id FROM chat_state WHERE chat_id = ?",
    )
    .bind(chat_id.0)
    .fetch_optional(db)
    .await?;
    Ok(result.map(|r| r.last_list_message_id))
}

pub async fn update_last_list_message_id(
    db: &Pool<Sqlite>,
    chat_id: ChatId,
    message_id: MessageId,
) -> Result<()> {
    tracing::debug!(
        chat_id = chat_id.0,
        message_id = message_id.0,
        "Updating last list message id",
    );
    sqlx::query(
        "INSERT INTO chat_state (chat_id, last_list_message_id) VALUES (?, ?) \
         ON CONFLICT(chat_id) DO UPDATE SET last_list_message_id = excluded.last_list_message_id",
    )
    .bind(chat_id.0)
    .bind(message_id.0)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn clear_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Clearing last list message id");
    sqlx::query("DELETE FROM chat_state WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}
