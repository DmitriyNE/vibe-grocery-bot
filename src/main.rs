use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use std::env;

#[derive(sqlx::FromRow, Debug)]
struct Item {
    id: i64,
    chat_id: i64,
    text: String,
    done: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();

    let token = env::var("BOT_TOKEN")?;
    let bot = Bot::new(token);

    // DB pool — file will be created if absent
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://shopping.db")
        .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS items(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id INTEGER NOT NULL,
            text TEXT NOT NULL,
            done BOOLEAN NOT NULL DEFAULT 0
        )",
    )
    .execute(&db)
    .await?;

    let handler = dptree::entry()
        // /start or /help → help text
        .branch(Update::filter_message().filter_command::<String>().endpoint(help))
        // plain text → add new item
        .branch(Update::filter_message().filter_text().endpoint(add_item))
        // callback queries → toggle done
        .branch(Update::filter_callback_query().endpoint(toggle_item))
        .branch(Update::filter_message().filter_command::<()>("list").endpoint(send_list))
        .branch(Update::filter_message().filter_command::<()>("archive").endpoint(archive));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

// ==== Handlers ====

async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any line → it’s added.\n\
         Tap ✅ to mark done.\n\
         /list shows current list.\n\
         /archive dumps checked items & removes them.",
    )
    .await?;
    Ok(())
}

async fn add_item(bot: Bot, msg: Message, db: SqlitePool) -> Result<()> {
    let chat = msg.chat.id.0;
    let text = msg.text().unwrap().trim();
    if text.is_empty() {
        return Ok(());
    }
    sqlx::query("INSERT INTO items (chat_id, text) VALUES (?, ?)")
        .bind(chat)
        .bind(text)
        .execute(&db)
        .await?;
    send_list_impl(&bot, chat, &db).await?;
    Ok(())
}

async fn toggle_item(bot: Bot, q: CallbackQuery, db: SqlitePool) -> Result<()> {
    if let Some(data) = q.data {
        let id: i64 = data.parse()?;
        sqlx::query("UPDATE items SET done = NOT done WHERE id = ?")
            .bind(id)
            .execute(&db)
            .await?;
        if let Some(msg) = &q.message {
            send_list_impl(&bot, msg.chat.id.0, &db).await?;
        }
        bot.answer_callback_query(q.id).await?;
    }
    Ok(())
}

async fn send_list(bot: Bot, msg: Message, db: SqlitePool) -> Result<()> {
    send_list_impl(&bot, msg.chat.id.0, &db).await
}

async fn send_list_impl(bot: &Bot, chat_id: i64, db: &SqlitePool) -> Result<()> {
    let rows: Vec<Item> = sqlx::query_as("SELECT * FROM items WHERE chat_id = ?")
        .bind(chat_id)
        .fetch_all(db)
        .await?;
    if rows.is_empty() {
        bot.send_message(chat_id, "List is empty.").await?;
        return Ok(());
    }
    let mut text = String::new();
    let mut keyboard = vec![];
    for item in rows {
        let mark = if item.done { "✅" } else { "⬜️" };
        text.push_str(&format!("{} {}\n", mark, item.text));
        keyboard.push(vec![InlineKeyboardButton::callback(
            mark.to_string(),
            item.id.to_string(),
        )]);
    }
    let kb = InlineKeyboardMarkup::new(keyboard);
    bot.send_message(chat_id, text).reply_markup(kb).await?;
    Ok(())
}

async fn archive(bot: Bot, msg: Message, db: SqlitePool) -> Result<()> {
    let chat = msg.chat.id.0;
    // fetch + delete in one transaction
    let mut tx = db.begin().await?;
    let done: Vec<String> = sqlx::query_as::<_, Item>(
        "SELECT * FROM items WHERE chat_id = ? AND done = 1",
    )
    .bind(chat)
    .fetch_all(&mut tx)
    .await?
    .into_iter()
    .map(|it| it.text)
    .collect();
    sqlx::query("DELETE FROM items WHERE chat_id = ? AND done = 1")
        .bind(chat)
        .execute(&mut tx)
        .await?;
    tx.commit().await?;

    if done.is_empty() {
        bot.send_message(chat, "Nothing to archive (no ✔ items).").await?;
    } else {
        let out = done.join(", ");
        bot.send_message(chat, format!("Archived: {}", out)).await?;
    }
    send_list_impl(&bot, chat, &db).await?;
    Ok(())
}
