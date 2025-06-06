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
        #[command(description = "finalize and archive the current list, starting a new one.")]
        Archive,
        #[command(description = "enter mode to delete items from the list.")]
        Delete,
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

/// Deletes a single item from the database by its ID.
async fn delete_item(db: &Pool<Sqlite>, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Deletes all items for a given chat, effectively clearing the active list.
async fn delete_all_items(db: &Pool<Sqlite>, chat_id: ChatId) -> Result<()> {
    sqlx::query("DELETE FROM items WHERE chat_id = ?")
        .bind(chat_id.0)
        .execute(db)
        .await?;
    Ok(())
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

/// Removes the tracked message ID for a chat, preventing an archived message from being deleted.
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

/// Sends the help message.
async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any text to add it to your shopping list. Each line will be a new item.\n\
         You can tap the checkbox button next to an item to mark it as bought.\n\n\
         <b>Commands:</b>\n\
         /list - Show the current shopping list.\n\
         /archive - Finalize and archive the current list, starting a new one.\n\
         /delete - Show a temporary panel to delete items from the list.",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}

/// Generates the text and keyboard for the normal shopping list view.
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

/// Generates the text and keyboard for the delete mode view.
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

    // Add the "Done" button to exit delete mode
    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
        "✅ Done Deleting",
        "delete_done",
    )]);

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
/// This function now takes message_id and chat_id to avoid needing the full Message object.
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

/// Archives the entire current list, making it static and starting a new one.
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

/// Sends a temporary, user-specific message with delete buttons.
async fn enter_delete_mode(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    // Immediately delete the user's /delete command to keep the chat clean.
    let _ = bot.delete_message(msg.chat.id, msg.id).await;

    // Check if there is an active list to edit.
    if get_last_list_message_id(db, msg.chat.id).await?.is_none() {
        let sent_msg = bot
            .send_message(msg.chat.id, "There is no active list to edit.")
            .await?;
        // Delete this notification after a few seconds.
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let _ = bot.delete_message(sent_msg.chat.id, sent_msg.id).await;
        });
        return Ok(());
    }

    let items = list_items(db, msg.chat.id).await?;
    if items.is_empty() {
        return Ok(()); // No items to delete, so do nothing.
    }

    let (text, keyboard) = format_delete_list(&items);

    // Send a new, temporary message with the delete panel.
    bot.send_message(msg.chat.id, text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

/// Handles all callback queries (button presses).
async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        // --- Delete Mode Logic ---
        if let Some(id_str) = data.strip_prefix("delete_") {
            if id_str == "done" {
                // The user is done, so delete the temporary delete panel.
                let _ = bot.delete_message(msg.chat.id, msg.id).await;
            } else if let Ok(id) = id_str.parse::<i64>() {
                // 1. Delete the item from the database.
                delete_item(&db, id).await?;

                // 2. Refresh the main shared list.
                if let Some(main_list_id) = get_last_list_message_id(&db, msg.chat.id).await? {
                    update_list_message(&bot, msg.chat.id, MessageId(main_list_id), &db).await?;
                }

                // 3. Refresh the temporary delete panel with the remaining items.
                let items = list_items(&db, msg.chat.id).await?;
                if items.is_empty() {
                    // If no items are left, just delete the panel.
                    let _ = bot.delete_message(msg.chat.id, msg.id).await;
                } else {
                    let (text, keyboard) = format_delete_list(&items);
                    let _ = bot
                        .edit_message_text(msg.chat.id, msg.id, text)
                        .reply_markup(keyboard)
                        .await;
                }
            }
        // --- Normal Mode (Toggle) Logic ---
        } else if let Ok(id) = data.parse::<i64>() {
            toggle_item(&db, id).await?;
            // Update the main list message directly.
            update_list_message(&bot, msg.chat.id, msg.id, &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
