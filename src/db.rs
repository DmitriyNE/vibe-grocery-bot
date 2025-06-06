// Database related types and functions

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::collections::HashSet;
use teloxide::types::{ChatId, MessageId};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct Item {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

#[derive(sqlx::FromRow)]
struct ChatState {
    last_list_message_id: i32,
}

#[derive(sqlx::FromRow)]
struct DeleteSessionRow {
    chat_id: i64,
    selected: String,
    notice_chat_id: Option<i64>,
    notice_message_id: Option<i32>,
    dm_message_id: Option<i32>,
}

pub struct DeleteSession {
    pub chat_id: ChatId,
    pub selected: HashSet<i64>,
    pub notice: Option<(ChatId, MessageId)>,
    pub dm_message_id: Option<MessageId>,
}

fn parse_selected(s: &str) -> HashSet<i64> {
    s.split(',').filter_map(|p| p.parse::<i64>().ok()).collect()
}

fn join_selected(set: &HashSet<i64>) -> String {
    let mut ids: Vec<i64> = set.iter().copied().collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub async fn connect_db(db_url: &str) -> Result<Pool<Sqlite>> {
    Ok(SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?)
}

pub async fn add_item(db: &Pool<Sqlite>, chat_id: ChatId, text: &str) -> Result<()> {
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat_id.0)
        .bind(text)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<Item>> {
    sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
        .bind(chat_id.0)
        .fetch_all(db)
        .await
        .map_err(Into::into)
}

pub async fn toggle_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_all_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Option<i32>> {
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
    sqlx::query("DELETE FROM chat_state WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn init_delete_session(db: &Pool<Sqlite>, user_id: i64, chat_id: ChatId) -> Result<()> {
    sqlx::query(
        "INSERT INTO delete_session (user_id, chat_id, selected) VALUES (?, ?, '') \
         ON CONFLICT(user_id) DO UPDATE SET chat_id=excluded.chat_id, selected='', notice_chat_id=NULL, notice_message_id=NULL, dm_message_id=NULL",
    )
    .bind(user_id)
    .bind(chat_id.0)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_delete_selection(
    db: &Pool<Sqlite>,
    user_id: i64,
    selected: &HashSet<i64>,
) -> Result<()> {
    let joined = join_selected(selected);
    sqlx::query("UPDATE delete_session SET selected = ? WHERE user_id = ?")
        .bind(joined)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_delete_notice(
    db: &Pool<Sqlite>,
    user_id: i64,
    chat_id: ChatId,
    message_id: MessageId,
) -> Result<()> {
    sqlx::query(
        "UPDATE delete_session SET notice_chat_id = ?, notice_message_id = ? WHERE user_id = ?",
    )
    .bind(chat_id.0)
    .bind(message_id.0)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_delete_dm_message(
    db: &Pool<Sqlite>,
    user_id: i64,
    message_id: MessageId,
) -> Result<()> {
    sqlx::query("UPDATE delete_session SET dm_message_id = ? WHERE user_id = ?")
        .bind(message_id.0)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_delete_session(db: &Pool<Sqlite>, user_id: i64) -> Result<Option<DeleteSession>> {
    if let Some(row) = sqlx::query_as::<_, DeleteSessionRow>(
        "SELECT chat_id, selected, notice_chat_id, notice_message_id, dm_message_id FROM delete_session WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    {
        let notice = match (row.notice_chat_id, row.notice_message_id) {
            (Some(c), Some(m)) => Some((ChatId(c), MessageId(m))),
            _ => None,
        };
        Ok(Some(DeleteSession {
            chat_id: ChatId(row.chat_id),
            selected: parse_selected(&row.selected),
            notice,
            dm_message_id: row.dm_message_id.map(MessageId),
        }))
    } else {
        Ok(None)
    }
}

pub async fn clear_delete_session(db: &Pool<Sqlite>, user_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM delete_session WHERE user_id = ?")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}
