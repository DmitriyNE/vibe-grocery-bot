use anyhow::Result;
use dotenvy::dotenv;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::env; // Import the standard library's env module
use teloxide::{
    prelude::*,
    requests::Requester,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId},
    utils::command::BotCommands,
};

// ──────────────────────────────────────────────────────────────
// Main application setup
// ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
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

    // --- THIS IS THE FIX ---
    // Ensure the connection string has the 'create if not exists' flag.
    // This is crucial for the first run on a new volume, as it tells SQLite
    // to create the database file if it's missing.
    if db_url.starts_with("sqlite:") && !db_url.contains("mode=") {
        db_url.push_str("?mode=rwc");
    }

    tracing::info!("Connecting to database at: {}", &db_url);

    let db: Pool<Sqlite> = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    tracing::info!("Database connection successful.");

    // --- Database Schema ---
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS items(
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id   INTEGER NOT NULL,
            text      TEXT    NOT NULL,
            done      BOOLEAN NOT NULL DEFAULT 0
        )",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_state(
            chat_id                 INTEGER PRIMARY KEY,
            last_list_message_id    INTEGER
        )",
    )
    .execute(&db)
    .await?;

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
// DB Helpers
// ──────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct Item {
    id: i64,
    text: String,
    done: bool,
}

async fn add_item(db: &Pool<Sqlite>, chat_id: ChatId, text: &str) -> Result<()> {
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat_id.0)
        .bind(text)
        .execute(db)
        .await?;
    Ok(())
}

async fn list_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<Item>> {
    sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
        .bind(chat_id.0)
        .fetch_all(db)
        .await
        .map_err(Into::into)
}

async fn toggle_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

async fn delete_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

async fn delete_all_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ChatState {
    last_list_message_id: i32,
}

async fn get_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Option<i32>> {
    let result = sqlx::query_as::<_, ChatState>(
        "SELECT last_list_message_id FROM chat_state WHERE chat_id = ?",
    )
    .bind(chat_id.0)
    .fetch_optional(db)
    .await?;
    Ok(result.map(|r| r.last_list_message_id))
}

async fn update_last_list_message_id(
    db: &Pool<Sqlite>,
    chat_id: ChatId,
    message_id: MessageId,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO chat_state (chat_id, last_list_message_id) VALUES (?, ?)
         ON CONFLICT(chat_id) DO UPDATE SET last_list_message_id = excluded.last_list_message_id",
    )
    .bind(chat_id.0)
    .bind(message_id.0)
    .execute(db)
    .await?;
    Ok(())
}

async fn clear_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    sqlx::query("DELETE FROM chat_state WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Bot Handlers & Helpers
// ──────────────────────────────────────────────────────────────

async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any text to add it to your shopping list. Each line will be a new item.\n\
         You can tap the checkbox button next to an item to mark it as bought.\n\n\
         <b>Commands:</b>\n\
         /list - Show the current shopping list.\n\
         /archive - Finalize and archive the current list, starting a new one.\n\
         /delete - Show a temporary panel to delete items from the list.\n\
         /nuke - Completely delete the current list.",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}

fn format_list(items: &[Item]) -> (String, InlineKeyboardMarkup) {
    let mut text = String::new();
    let mut keyboard_buttons = Vec::new();

    for item in items {
        let mark = if item.done { "✅" } else { "🛒" };
        let button_text = if item.done {
            format!("✅ {}", item.text)
        } else {
            item.text.clone()
        };
        text.push_str(&format!("{} {}\n", mark, item.text));
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            item.id.to_string(),
        )]);
    }

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

fn format_delete_list(items: &[Item]) -> (String, InlineKeyboardMarkup) {
    let text = "Tap an item to delete it. Tap 'Done' when finished.".to_string();
    let mut keyboard_buttons = Vec::new();

    for item in items {
        let button_text = format!("❌ {}", item.text);
        let callback_data = format!("delete_{}", item.id);
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            callback_data,
        )]);
    }

    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
        "✅ Done Deleting",
        "delete_done",
    )]);

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

async fn add_items_from_text(bot: Bot, msg: Message, db: Pool<Sqlite>) -> Result<()> {
    if let Some(text) = msg.text() {
        let mut items_added_count = 0;
        for line in text.lines() {
            if line.trim() == "--- Archived List ---" {
                continue;
            }

            let cleaned_line = line.trim_start_matches(['✅', '🛒']).trim();

            if !cleaned_line.is_empty() {
                add_item(&db, msg.chat.id, cleaned_line).await?;
                items_added_count += 1;
            }
        }

        if items_added_count > 0 {
            tracing::info!(
                "Added {} item(s) for chat {}",
                items_added_count,
                msg.chat.id
            );
            send_list(bot, msg.chat.id, &db).await?;
        }
    }
    Ok(())
}

async fn send_list(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    if let Some(message_id) = get_last_list_message_id(db, chat_id).await? {
        let _ = bot.delete_message(chat_id, MessageId(message_id)).await;
    }

    let items = list_items(db, chat_id).await?;

    if items.is_empty() {
        let sent_msg = bot
            .send_message(
                chat_id,
                "Your shopping list is empty! Send any message to add an item.",
            )
            .await?;
        update_last_list_message_id(db, chat_id, sent_msg.id).await?;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);

    let sent_msg = bot
        .send_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;

    update_last_list_message_id(db, chat_id, sent_msg.id).await?;

    Ok(())
}

async fn update_list_message(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    db: &Pool<Sqlite>,
) -> Result<()> {
    let items = list_items(db, chat_id).await?;

    if items.is_empty() {
        let _ = bot
            .edit_message_text(chat_id, message_id, "List is now empty!")
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);

    let _ = bot
        .edit_message_text(chat_id, message_id, text)
        .reply_markup(keyboard)
        .await;

    Ok(())
}

async fn archive(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    let last_message_id = match get_last_list_message_id(db, chat_id).await? {
        Some(id) => id,
        None => {
            bot.send_message(chat_id, "There is no active list to archive.")
                .await?;
            return Ok(());
        }
    };

    let items = list_items(db, chat_id).await?;
    if items.is_empty() {
        bot.send_message(chat_id, "There is no active list to archive.")
            .await?;
        return Ok(());
    }

    let (final_text, _) = format_list(&items);
    let archived_text = format!("--- Archived List ---\n{}", final_text);

    let _ = bot
        .edit_message_text(chat_id, MessageId(last_message_id), archived_text)
        .reply_markup(InlineKeyboardMarkup::new(
            Vec::<Vec<InlineKeyboardButton>>::new(),
        ))
        .await;

    delete_all_items(db, chat_id).await?;
    clear_last_list_message_id(db, chat_id).await?;

    bot.send_message(chat_id, "List archived! Send a message to start a new one.")
        .await?;

    Ok(())
}

async fn enter_delete_mode(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    let _ = bot.delete_message(msg.chat.id, msg.id).await;

    if get_last_list_message_id(db, msg.chat.id).await?.is_none() {
        let sent_msg = bot
            .send_message(msg.chat.id, "There is no active list to edit.")
            .await?;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let _ = bot.delete_message(sent_msg.chat.id, sent_msg.id).await;
        });
        return Ok(());
    }

    let items = list_items(db, msg.chat.id).await?;
    if items.is_empty() {
        return Ok(());
    }

    let (text, keyboard) = format_delete_list(&items);

    bot.send_message(msg.chat.id, text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn nuke_list(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    let _ = bot.delete_message(msg.chat.id, msg.id).await;

    if let Some(list_message_id) = get_last_list_message_id(db, msg.chat.id).await? {
        let _ = bot
            .delete_message(msg.chat.id, MessageId(list_message_id))
            .await;
    }

    delete_all_items(db, msg.chat.id).await?;
    clear_last_list_message_id(db, msg.chat.id).await?;

    let confirmation = bot
        .send_message(msg.chat.id, "The active list has been nuked.")
        .await?;
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let _ = bot
            .delete_message(confirmation.chat.id, confirmation.id)
            .await;
    });

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Some(id_str) = data.strip_prefix("delete_") {
            if id_str == "done" {
                let _ = bot.delete_message(msg.chat.id, msg.id).await;
            } else if let Ok(id) = id_str.parse::<i64>() {
                delete_item(&db, id).await?;

                if let Some(main_list_id) = get_last_list_message_id(&db, msg.chat.id).await? {
                    update_list_message(&bot, msg.chat.id, MessageId(main_list_id), &db).await?;
                }

                let items = list_items(&db, msg.chat.id).await?;
                if items.is_empty() {
                    let _ = bot.delete_message(msg.chat.id, msg.id).await;
                } else {
                    let (text, keyboard) = format_delete_list(&items);
                    let _ = bot
                        .edit_message_text(msg.chat.id, msg.id, text)
                        .reply_markup(keyboard)
                        .await;
                }
            }
        } else if let Ok(id) = data.parse::<i64>() {
            toggle_item(&db, id).await?;
            update_list_message(&bot, msg.chat.id, msg.id, &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
