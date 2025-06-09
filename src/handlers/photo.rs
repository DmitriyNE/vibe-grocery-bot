use anyhow::Result;
use futures_util::StreamExt;
use sqlx::{Pool, Sqlite};
use teloxide::{net::Download, prelude::*};

use crate::ai::vision::parse_photo_items;
use crate::db::add_item;
use crate::text_utils::capitalize_first;

use super::list::send_list;
use crate::ai::config::AiConfig;

pub async fn add_items_from_photo(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    stt: Option<AiConfig>,
) -> Result<()> {
    let Some(config) = stt else {
        return Ok(());
    };

    let photo_sizes = match msg.photo() {
        Some(p) => p,
        None => return Ok(()),
    };
    let Some(file_id) = photo_sizes
        .iter()
        .max_by_key(|p| p.file.size)
        .map(|p| &p.file.id)
    else {
        tracing::debug!("photo had no usable sizes");
        return Ok(());
    };

    tracing::debug!(model = %config.vision_model, "parsing photo with OpenAI vision");
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

    async fn init_db() -> Pool<Sqlite> {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE items(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    text TEXT NOT NULL,\n    done BOOLEAN NOT NULL DEFAULT 0\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE chat_state(\n    chat_id INTEGER PRIMARY KEY,\n    last_list_message_id INTEGER\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE delete_session(\n    user_id INTEGER PRIMARY KEY,\n    chat_id INTEGER NOT NULL,\n    selected TEXT NOT NULL DEFAULT '',\n    notice_chat_id INTEGER,\n    notice_message_id INTEGER,\n    dm_message_id INTEGER\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn photo_with_no_sizes_returns_ok() {
        let db = init_db().await;
        let bot = Bot::new("test");
        let json = r#"{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"photo":[]}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        let stt = Some(AiConfig {
            api_key: "k".into(),
            stt_model: "m".into(),
            gpt_model: "g".into(),
            vision_model: "v".into(),
        });

        let res = add_items_from_photo(bot, msg, db, stt).await;
        assert!(res.is_ok());
    }
}
        tracing::debug!("photo had no usable sizes");
        return Ok(());
    };

    let file = bot.get_file(file_id).await?;
    let mut bytes = Vec::new();
    let mut stream = bot.download_file_stream(&file.path);
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
    }

    let items = match parse_photo_items(&config.api_key, &config.vision_model, &bytes).await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("photo parsing failed: {}", err);
            Vec::new()
        }
    };

    let mut added = 0;
    for item in items {
        let cap = capitalize_first(&item);
        add_item(&db, msg.chat.id, &cap).await?;
        added += 1;
    }

    if added > 0 {
        tracing::info!(
            "Added {} item(s) from photo for chat {}",
            added,
            msg.chat.id
        );
        send_list(bot, msg.chat.id, &db).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

    async fn init_db() -> Pool<Sqlite> {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE items(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    text TEXT NOT NULL,\n    done BOOLEAN NOT NULL DEFAULT 0\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE chat_state(\n    chat_id INTEGER PRIMARY KEY,\n    last_list_message_id INTEGER\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE delete_session(\n    user_id INTEGER PRIMARY KEY,\n    chat_id INTEGER NOT NULL,\n    selected TEXT NOT NULL DEFAULT '',\n    notice_chat_id INTEGER,\n    notice_message_id INTEGER,\n    dm_message_id INTEGER\n)",
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn photo_with_no_sizes_returns_ok() {
        let db = init_db().await;
        let bot = Bot::new("test");
        let json = r#"{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"photo":[]}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        let stt = Some(SttConfig {
            api_key: "k".into(),
            model: "m".into(),
            gpt_model: "g".into(),
        });

        let res = add_items_from_photo(bot, msg, db, stt).await;
        assert!(res.is_ok());
    }
}
