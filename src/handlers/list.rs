use crate::db::{Database, Item};
use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId},
};

use super::list_service::ListService;

pub fn format_list(items: &[Item]) -> (String, InlineKeyboardMarkup) {
    let mut text = String::new();
    let mut keyboard_buttons = Vec::new();

    let all_done = items.iter().all(|i| i.done);

    for item in items {
        let (mark, button_text) = if all_done {
            ("✅", format!("✅ {}", item.text))
        } else if item.done {
            ("☑️", format!("☑️ {}", item.text))
        } else {
            ("⬜", format!("⬜ {}", item.text))
        };
        text.push_str(&format!("{} {}\n", mark, item.text));
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            item.id.to_string(),
        )]);
    }

    if all_done && !items.is_empty() {
        tracing::debug!("List fully checked out");
    }

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

pub fn format_plain_list(items: &[Item]) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&format!("• {}\n", item.text));
    }
    text
}

pub async fn send_list(bot: Bot, chat_id: ChatId, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Sending list");
    ListService::new(db).send_list(bot, chat_id).await
}

pub async fn share_list(bot: Bot, chat_id: ChatId, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Sharing list");
    ListService::new(db).share_list(bot, chat_id).await
}

pub async fn update_list_message(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    db: &Database,
) -> Result<()> {
    tracing::debug!(
        chat_id = chat_id.0,
        message_id = message_id.0,
        "Updating list message",
    );
    ListService::new(db)
        .update_message(bot, chat_id, message_id)
        .await
}

pub async fn archive(bot: Bot, chat_id: ChatId, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Archiving list");
    ListService::new(db).archive(bot, chat_id).await
}

pub async fn nuke_list(bot: Bot, msg: Message, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = msg.chat.id.0, "Nuking list");
    ListService::new(db).nuke(bot, msg).await
}

pub async fn insert_items<I>(bot: Bot, chat_id: ChatId, db: &Database, items: I) -> Result<usize>
where
    I: IntoIterator<Item = String>,
{
    let mut added = 0usize;
    for item in items {
        db.add_item(chat_id, &item).await?;
        added += 1;
    }

    if added > 0 {
        tracing::debug!(chat_id = chat_id.0, added, "Inserted items");
        send_list(bot, chat_id, db).await?;
    } else {
        tracing::debug!(chat_id = chat_id.0, "No items inserted");
    }
    Ok(added)
}
