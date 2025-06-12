use crate::db::{ChatKey, Database, Item};
use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup},
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

pub async fn insert_items<I>(bot: Bot, chat_id: ChatId, db: &Database, items: I) -> Result<usize>
where
    I: IntoIterator<Item = String>,
{
    let mut added = 0usize;
    for item in items {
        db.add_item(ChatKey(chat_id.0), &item).await?;
        added += 1;
    }

    if added > 0 {
        tracing::debug!(chat_id = chat_id.0, added, "Inserted items");
        ListService::new(db).send_list(bot, chat_id).await?;
    } else {
        tracing::debug!(chat_id = chat_id.0, "No items inserted");
    }
    Ok(added)
}
