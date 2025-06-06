use anyhow::Result;
use dotenvy::dotenv;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
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
    // Load .env file if it exists
    dotenv().ok();

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting grocery list bot...");

    let bot = Bot::from_env();

    // --- SQLite Pool ---
    let db: Pool<Sqlite> = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:shopping.db?mode=rwc")
        .await?;

    tracing::info!("Database connection successful.");

    // --- Database Schema ---
    // Create the 'items' table for the shopping list.
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

    // Create the 'chat_state' table to track the last list message per chat.
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
        #[command(description = "clear all checked items from the list.")]
        Archive,
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

/// Adds a single item to the database.
async fn add_item(db: &Pool<Sqlite>, chat_id: ChatId, text: &str) -> Result<()> {
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat_id.0)
        .bind(text)
        .execute(db)
        .await?;
    Ok(())
}

/// Retrieves all items for a given chat.
async fn list_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<Item>> {
    sqlx::query_as("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
        .bind(chat_id.0)
        .fetch_all(db)
        .await
        .map_err(Into::into)
}

/// Toggles the 'done' status of an item.
async fn toggle_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Deletes all 'done' items for a chat and returns their text.
async fn purge_done(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<String>> {
    #[derive(sqlx::FromRow)]
    struct ArchivedText {
        text: String,
    }

    let archived_items =
        sqlx::query_as::<_, ArchivedText>("SELECT text FROM items WHERE chat_id = ? AND done = 1")
            .bind(chat_id.0)
            .fetch_all(db)
            .await?;

    let texts = archived_items.into_iter().map(|r| r.text).collect();

    sqlx::query("DELETE FROM items WHERE chat_id = ? AND done = 1")
        .bind(chat_id.0)
        .execute(db)
        .await?;

    Ok(texts)
}

// Struct to fetch the last message ID from the chat_state table.
#[derive(sqlx::FromRow)]
struct ChatState {
    last_list_message_id: i32,
}

/// Retrieves the ID of the last list message sent to a chat.
async fn get_last_list_message_id(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Option<i32>> {
    let result = sqlx::query_as::<_, ChatState>(
        "SELECT last_list_message_id FROM chat_state WHERE chat_id = ?",
    )
    .bind(chat_id.0)
    .fetch_optional(db)
    .await?;
    Ok(result.map(|r| r.last_list_message_id))
}

/// Stores the ID of the latest list message for a chat.
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

// ──────────────────────────────────────────────────────────────
// Bot Handlers & Helpers
// ──────────────────────────────────────────────────────────────

/// Sends the help message.
async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any text to add it to your shopping list. Each line will be a new item.\n\
         You can tap the checkbox button next to an item to mark it as bought.\n\n\
         <b>Commands:</b>\n\
         /list - Show the current shopping list.\n\
         /archive - Clear all checked items from the list.",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}

/// Generates the text and keyboard for the shopping list.
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

/// Parses a message, adding each line as a separate item.
async fn add_items_from_text(bot: Bot, msg: Message, db: Pool<Sqlite>) -> Result<()> {
    if let Some(text) = msg.text() {
        let mut items_added_count = 0;
        for line in text.lines() {
            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                add_item(&db, msg.chat.id, trimmed_line).await?;
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

/// Sends a new message with the shopping list, deleting the previous one.
async fn send_list(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    // 1. Delete the old list message, if one exists.
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

    // 2. Send the new list message.
    let sent_msg = bot
        .send_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;

    // 3. Store the ID of the new message.
    update_last_list_message_id(db, chat_id, sent_msg.id).await?;

    Ok(())
}

/// Atomically edits an existing message to update the list.
async fn update_list_message(bot: Bot, msg: &Message, db: &Pool<Sqlite>) -> Result<()> {
    let items = list_items(db, msg.chat.id).await?;

    // This block handles the case where the list becomes empty.
    if items.is_empty() {
        let _ = bot
            .edit_message_text(msg.chat.id, msg.id, "List is now empty!")
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);

    // Atomically edit the message text and keyboard in a single API call.
    // This prevents the keyboard from flickering.
    let _ = bot
        .edit_message_text(msg.chat.id, msg.id, text)
        .reply_markup(keyboard)
        .await;

    Ok(())
}

/// Archives completed items and sends an updated list.
async fn archive(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    let archived = purge_done(db, chat_id).await?;

    if archived.is_empty() {
        bot.send_message(chat_id, "Nothing to archive (no checked items).")
            .await?;
    } else {
        bot.send_message(chat_id, format!("Archived: {}", archived.join(", ")))
            .await?;
    }

    send_list(bot, chat_id, db).await?;
    Ok(())
}

/// Handles callback queries from inline button presses.
async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Ok(id) = data.parse::<i64>() {
            toggle_item(&db, id).await?;
            update_list_message(bot.clone(), &msg, &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
