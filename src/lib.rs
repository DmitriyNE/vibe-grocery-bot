use anyhow::Result;
use dotenvy::dotenv;
use sqlx::{Pool, Sqlite};
use std::env; // Import the standard library's env module
use teloxide::{prelude::*, utils::command::BotCommands};

pub mod ai;
mod db;
mod handlers;
mod text_utils;
pub use handlers::{format_delete_list, format_list, format_plain_list};
pub use text_utils::{capitalize_first, parse_item_line};

    let db_url = db::prepare_sqlite_url(&db_url);
    if url.starts_with("sqlite:") && !url.contains("mode=") && !url.contains(":memory:") {
        if url.contains('?') {
            format!("{url}&mode=rwc")
        } else {
            format!("{url}?mode=rwc")
        }
    } else {
        url.to_string()
    }
}

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

    tracing::info!("Starting list bot...");

    let bot = Bot::from_env();

    // Optional OpenAI speech-to-text configuration
    let stt_config = match env::var("OPENAI_API_KEY") {
        Ok(key) => Some(crate::ai::stt::SttConfig {
            api_key: key,
            model: env::var("OPENAI_STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string()),
        }),
        Err(_) => None,
    };

    // --- SQLite Pool ---
    // Read the database URL from the environment, with a fallback for local dev.
    let db_url = env::var("DB_URL").unwrap_or_else(|_| "sqlite:shopping.db".to_string());
    let db_url = db::prepare_sqlite_url(&db_url);

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
        #[command(description = "send the list as plain text for copying.")]
        Share,
        #[command(description = "completely delete the current list.")]
        Nuke,
        #[command(description = "parse items from the given text using GPT.")]
        Parse,
    }

    // --- Handler Setup ---
    let handler = dptree::entry()
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.voice().is_some())
                        .endpoint(add_items_from_voice),
                )
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.photo().is_some())
                        .endpoint(add_items_from_photo),
                )
                .branch(dptree::entry().filter_command::<Command>().endpoint(
                    |bot: Bot,
                     msg: Message,
                     cmd: Command,
                     db: Pool<Sqlite>,
                     stt_config: Option<crate::ai::stt::SttConfig>| async move {
                        match cmd {
                            Command::Start | Command::Help => help(bot, msg).await?,
                            Command::List => send_list(bot, msg.chat.id, &db).await?,
                            Command::Archive => archive(bot, msg.chat.id, &db).await?,
                            Command::Delete => enter_delete_mode(bot, msg, &db).await?,
                            Command::Share => share_list(bot, msg.chat.id, &db).await?,
                            Command::Nuke => nuke_list(bot, msg, &db).await?,
                            Command::Parse => {
                                add_items_from_parsed_text(bot, msg, db, stt_config).await?
                            }
                        }
                        Ok(())
                    },
                ))
                .branch(dptree::endpoint(add_items_from_text)),
        );

    // --- Dispatcher ---
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db, stt_config])
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

        assert_eq!(
            prepare_sqlite_url("sqlite:items.db"),
            "sqlite:items.db?mode=rwc"
        );
    }

    #[test]
    fn prepare_sqlite_url_with_query() {
        assert_eq!(
            prepare_sqlite_url("sqlite:items.db?cache=shared"),
            "sqlite:items.db?cache=shared&mode=rwc"
        );
    }

    #[test]
    fn prepare_sqlite_url_existing_mode() {
        assert_eq!(
            prepare_sqlite_url("sqlite:items.db?mode=ro"),
            "sqlite:items.db?mode=ro"
        );
    }

    #[test]
    fn prepare_sqlite_url_memory() {
        assert_eq!(prepare_sqlite_url("sqlite::memory:"), "sqlite::memory:");
    }
}
