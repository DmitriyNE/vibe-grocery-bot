use anyhow::Result;
use dotenvy::dotenv;
use sqlx::{Pool, Sqlite};
use std::env; // Import the standard library's env module
use teloxide::{prelude::*, utils::command::BotCommands};

mod db;
mod handlers;

pub use db::Item;
pub use handlers::{format_delete_list, format_list};

use handlers::{
    add_items_from_text, archive, callback_handler, enter_delete_mode, help, nuke_list, send_list,
};
// ──────────────────────────────────────────────────────────────
// Main application setup
// ──────────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    // Load .env file if it exists (for local development)
    dotenv().ok();

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting grocery list bot...");

    let bot = Bot::from_env();

    // --- SQLite Pool ---
    // Read the database URL from the environment, with a fallback for local dev.
    let mut db_url = env::var("DB_URL").unwrap_or_else(|_| "sqlite:shopping.db".to_string());

    // Ensure the connection string has the 'create if not exists' flag so the
    // database file is created on first run. If other parameters are present
    // append `&mode=rwc`, otherwise start a new query string.
    if db_url.starts_with("sqlite:") && !db_url.contains("mode=") {
        if db_url.contains('?') {
            db_url.push_str("&mode=rwc");
        } else {
            db_url.push_str("?mode=rwc");
        }
    }

    tracing::info!("Connecting to database at: {}", &db_url);

    let db = db::connect_db(&db_url).await?;

    tracing::info!("Database connection successful.");

    // --- Run Migrations ---
    // Use embedded SQLx migrations so the database schema stays up to date
    // without requiring manual setup.
    sqlx::migrate!("./migrations").run(&db).await?;

    // --- Command Enum ---
    #[derive(BotCommands, Clone)]
    #[command(
        rename_rule = "lowercase",
        description = "These commands are supported:"
    )]
    enum Command {
        #[command(description = "display this text.")]
        Start,
        #[command(description = "display this text.")]
        Help,
        #[command(description = "show the current shopping list.")]
        List,
        #[command(description = "finalize and archive the current list, starting a new one.")]
        Archive,
        #[command(description = "show a temporary panel to delete items from the list.")]
        Delete,
        #[command(description = "completely delete the current list.")]
        Nuke,
    }

    // --- Handler Setup ---
    let handler = dptree::entry()
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(
            Update::filter_message()
                .branch(dptree::entry().filter_command::<Command>().endpoint(
                    |bot: Bot, msg: Message, cmd: Command, db: Pool<Sqlite>| async move {
                        match cmd {
                            Command::Start | Command::Help => help(bot, msg).await?,
                            Command::List => send_list(bot, msg.chat.id, &db).await?,
                            Command::Archive => archive(bot, msg.chat.id, &db).await?,
                            Command::Delete => enter_delete_mode(bot, msg, &db).await?,
                            Command::Nuke => nuke_list(bot, msg, &db).await?,
                        }
                        Ok(())
                    },
                ))
                .branch(dptree::endpoint(add_items_from_text)),
        );

    // --- Dispatcher ---
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
    use teloxide::types::MessageId;

    async fn init_db() -> Pool<Sqlite> {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE items(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id INTEGER NOT NULL,
                text TEXT NOT NULL,
                done BOOLEAN NOT NULL DEFAULT 0
            )",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE chat_state(
                chat_id INTEGER PRIMARY KEY,
                last_list_message_id INTEGER
            )",
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE delete_session(
                user_id INTEGER PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                selected TEXT NOT NULL DEFAULT '',
                notice_chat_id INTEGER,
                notice_message_id INTEGER,
                dm_message_id INTEGER
            )",
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn basic_item_flow() -> Result<()> {
        let db = init_db().await;
        let chat = ChatId(42);

        add_item(&db, chat, "Apples").await?;
        add_item(&db, chat, "Milk").await?;

        let mut items = list_items(&db, chat).await?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "Apples");
        assert!(!items[0].done);

        toggle_item(&db, items[0].id).await?;
        items = list_items(&db, chat).await?;
        assert!(items[0].done);

        delete_item(&db, items[1].id).await?;
        items = list_items(&db, chat).await?;
        assert_eq!(items.len(), 1);

        delete_all_items(&db, chat).await?;
        items = list_items(&db, chat).await?;
        assert!(items.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn last_message_id_roundtrip() -> Result<()> {
        let db = init_db().await;
        let chat = ChatId(1);

        assert!(get_last_list_message_id(&db, chat).await?.is_none());

        update_last_list_message_id(&db, chat, MessageId(99)).await?;
        assert_eq!(get_last_list_message_id(&db, chat).await?, Some(99));

        clear_last_list_message_id(&db, chat).await?;
        assert!(get_last_list_message_id(&db, chat).await?.is_none());

        Ok(())
    }
}
