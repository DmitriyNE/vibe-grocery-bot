use anyhow::Result;
use dotenvy::dotenv;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
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
    // The database will be created in a file named `shopping.db` if it doesn't exist.
    let db: Pool<Sqlite> = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://shopping.db")
        .await?;

    // --- Database Schema ---
    // Create the 'items' table if it doesn't already exist.
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

    // --- Command Enum ---
    // Defines the commands the bot understands.
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
    // This is the core of the bot's logic, routing different updates to different functions.
    let handler = dptree::entry()
        // Filter for callback queries (button presses)
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        // Filter for messages
        .branch(
            Update::filter_message()
                // Branch for commands
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
                // Default branch for any other text message (add item to list)
                .branch(dptree::endpoint(
                    |bot: Bot, msg: Message, db: Pool<Sqlite>| async move {
                        if let Some(text) = msg.text() {
                            add_item(&db, msg.chat.id, text.trim()).await?;
                            send_list(bot, msg.chat.id, &db).await?;
                        }
                        Ok(())
                    },
                )),
        );

    // --- Dispatcher ---
    // Runs the bot, passing updates to the handler chain.
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

// A struct to map the database row to.
#[derive(sqlx::FromRow)]
struct Item {
    id: i64,
    text: String,
    done: bool,
}

/// Adds a new item to the shopping list for a given chat.
async fn add_item(db: &Pool<Sqlite>, chat_id: ChatId, text: &str) -> Result<()> {
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat_id.0)
        .bind(text)
        .execute(db)
        .await?;
    Ok(())
}

/// Retrieves all non-archived items for a given chat.
async fn list_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<(i64, String, bool)>> {
    let items =
        sqlx::query_as::<_, Item>("SELECT id, text, done FROM items WHERE chat_id = ? ORDER BY id")
            .bind(chat_id.0)
            .fetch_all(db)
            .await?;

    Ok(items
        .into_iter()
        .map(|item| (item.id, item.text, item.done))
        .collect())
}

/// Toggles the 'done' status of an item by its ID.
async fn toggle_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Deletes all 'done' items for a chat and returns their text.
async fn purge_done(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<Vec<String>> {
    // A simple struct to hold just the text from the query result.
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

// ──────────────────────────────────────────────────────────────
// Bot Handlers & Helpers
// ──────────────────────────────────────────────────────────────

/// Sends the help message.
async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any text to add it to your shopping list.\n\
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
fn format_list(items: &[(i64, String, bool)]) -> (String, InlineKeyboardMarkup) {
    let mut text = String::new();
    let mut keyboard_buttons = Vec::new();

    for (id, item_text, done) in items {
        let mark = if *done { "✅" } else { "🛒" };
        let button_text = if *done {
            format!("✅ {}", item_text)
        } else {
            item_text.clone()
        };
        text.push_str(&format!("{} {}\n", mark, item_text));
        // Each button callback data is the item's database ID
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            id.to_string(),
        )]);
    }

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

/// Sends a new message with the current shopping list.
async fn send_list(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    let items = list_items(db, chat_id).await?;

    if items.is_empty() {
        bot.send_message(
            chat_id,
            "Your shopping list is empty! Send any message to add an item.",
        )
        .await?;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);

    bot.send_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

/// Edits an existing message to show the updated shopping list.
async fn update_list_message(bot: Bot, msg: &Message, db: &Pool<Sqlite>) -> Result<()> {
    let items = list_items(db, msg.chat.id).await?;

    if items.is_empty() {
        bot.edit_message_text(msg.chat.id, msg.id, "List is now empty!")
            .await?;
        bot.edit_message_reply_markup(msg.chat.id, msg.id)
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await?;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);

    // Use edit_message_text and edit_message_reply_markup to avoid sending a new message
    // A Timeout error can occur if the message content hasn't changed.
    let _ = bot.edit_message_text(msg.chat.id, msg.id, text).await;
    let _ = bot
        .edit_message_reply_markup(msg.chat.id, msg.id)
        .reply_markup(keyboard)
        .await;

    Ok(())
}

/// Archives completed items and informs the user.
async fn archive(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    let archived = purge_done(db, chat_id).await?;

    if archived.is_empty() {
        bot.send_message(chat_id, "Nothing to archive (no checked items).")
            .await?;
    } else {
        bot.send_message(chat_id, format!("Archived: {}", archived.join(", ")))
            .await?;
    }
    // After archiving, show the updated list
    send_list(bot, chat_id, db).await?;
    Ok(())
}

/// Handles callback queries from inline button presses.
async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    // Ensure we have a message and callback data to work with
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        // The data is the database ID of the item
        if let Ok(id) = data.parse::<i64>() {
            toggle_item(&db, id).await?;
            // Update the message in-place
            update_list_message(bot.clone(), &msg, &db).await?;
        }
    }
    // Tell Telegram we've handled the callback
    bot.answer_callback_query(q.id).await?;
    Ok(())
}
