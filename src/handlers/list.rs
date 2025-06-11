use crate::db::{Database, Item};
use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId},
};

use crate::messages::{
    ARCHIVED_LIST_HEADER, LIST_ARCHIVED, LIST_EMPTY, LIST_EMPTY_ADD_ITEM, LIST_NOW_EMPTY,
    LIST_NUKED, NO_ACTIVE_LIST_TO_ARCHIVE,
};

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
    if let Some(msg_id) = db.get_last_list_message_id(chat_id).await? {
        if let Err(err) = bot.delete_message(chat_id, MessageId(msg_id)).await {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = msg_id,
                "Failed to delete message",
            );
        }
    }

    let items = db.list_items(chat_id).await?;
    if items.is_empty() {
        let sent = bot.send_message(chat_id, LIST_EMPTY_ADD_ITEM).await?;
        db.update_last_list_message_id(chat_id, sent.id).await?;
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);
    let sent = bot
        .send_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;
    db.update_last_list_message_id(chat_id, sent.id).await?;
    Ok(())
}

pub async fn share_list(bot: Bot, chat_id: ChatId, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Sharing list");
    let items = db.list_items(chat_id).await?;
    if items.is_empty() {
        bot.send_message(chat_id, LIST_EMPTY).await?;
        return Ok(());
    }
    let text = format_plain_list(&items);
    bot.send_message(chat_id, text).await?;
    Ok(())
}

pub async fn update_message(
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
    let items = db.list_items(chat_id).await?;
    if items.is_empty() {
        if let Err(err) = bot
            .edit_message_text(chat_id, message_id, LIST_NOW_EMPTY)
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await
        {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = message_id.0,
                "Failed to edit message",
            );
        }
        return Ok(());
    }

    let (text, keyboard) = format_list(&items);
    if let Err(err) = bot
        .edit_message_text(chat_id, message_id, text)
        .reply_markup(keyboard)
        .await
    {
        tracing::warn!(
            error = %err,
            chat_id = chat_id.0,
            message_id = message_id.0,
            "Failed to edit message",
        );
    }
    Ok(())
}

pub async fn archive(bot: Bot, chat_id: ChatId, db: &Database) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Archiving list");
    let last_message_id = match db.get_last_list_message_id(chat_id).await? {
        Some(id) => id,
        None => {
            bot.send_message(chat_id, NO_ACTIVE_LIST_TO_ARCHIVE).await?;
            return Ok(());
        }
    };

    let items = db.list_items(chat_id).await?;
    if items.is_empty() {
        bot.send_message(chat_id, NO_ACTIVE_LIST_TO_ARCHIVE).await?;
        return Ok(());
    }

    let (final_text, _) = format_list(&items);
    let archived_text = format!("{ARCHIVED_LIST_HEADER}\n{}", final_text);

    if let Err(err) = bot
        .edit_message_text(chat_id, MessageId(last_message_id), archived_text)
        .reply_markup(InlineKeyboardMarkup::new(
            Vec::<Vec<InlineKeyboardButton>>::new(),
        ))
        .await
    {
        tracing::warn!(
            error = %err,
            chat_id = chat_id.0,
            message_id = last_message_id,
            "Failed to edit message",
        );
    }

    db.delete_all_items(chat_id).await?;
    db.clear_last_list_message_id(chat_id).await?;

    bot.send_message(chat_id, LIST_ARCHIVED).await?;
    Ok(())
}

pub async fn nuke(bot: Bot, msg: Message, db: &Database, delete_after_timeout: u64) -> Result<()> {
    tracing::debug!(chat_id = msg.chat.id.0, "Nuking list");
    if let Err(err) = bot.delete_message(msg.chat.id, msg.id).await {
        tracing::warn!(
            error = %err,
            chat_id = msg.chat.id.0,
            message_id = msg.id.0,
            "Failed to delete message",
        );
    }
    if let Some(list_message_id) = db.get_last_list_message_id(msg.chat.id).await? {
        if let Err(err) = bot
            .delete_message(msg.chat.id, MessageId(list_message_id))
            .await
        {
            tracing::warn!(
                error = %err,
                chat_id = msg.chat.id.0,
                message_id = list_message_id,
                "Failed to delete message",
            );
        }
    }
    db.delete_all_items(msg.chat.id).await?;
    db.clear_last_list_message_id(msg.chat.id).await?;
    let confirmation = bot.send_message(msg.chat.id, LIST_NUKED).await?;
    drop(crate::delete_after(
        bot.clone(),
        confirmation.chat.id,
        confirmation.id,
        delete_after_timeout,
    ));
    Ok(())
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
