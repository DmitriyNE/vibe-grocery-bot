use crate::db::Database;
use anyhow::Result;
use std::collections::HashSet;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId},
};

use crate::db::Item;
use crate::messages::{
    delete_dm_text, delete_user_selecting_text, DEFAULT_CHAT_NAME, DELETE_DM_FAILED,
    DELETE_DONE_LABEL, DELETE_SELECT_PROMPT, NO_ACTIVE_LIST_TO_EDIT,
};

use super::list::update_list_message;

pub fn format_delete_list(
    items: &[Item],
    selected: &HashSet<i64>,
) -> (String, InlineKeyboardMarkup) {
    let text = DELETE_SELECT_PROMPT.to_string();

    let mut keyboard_buttons = Vec::new();

    for item in items {
        let button_text = if selected.contains(&item.id) {
            format!("❌ {}", item.text)
        } else {
            format!("⬜ {}", item.text)
        };
        let callback_data = format!("delete_{}", item.id);
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            callback_data,
        )]);
    }

    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
        DELETE_DONE_LABEL,
        "delete_done",
    )]);

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

pub async fn enter_delete_mode(bot: Bot, msg: Message, db: &Database) -> Result<()> {
    tracing::debug!(
        chat_id = msg.chat.id.0,
        user_id = msg.from.as_ref().map(|u| u.id.0),
        "Entering delete mode",
    );
    if let Err(err) = bot.delete_message(msg.chat.id, msg.id).await {
        tracing::warn!(
            error = %err,
            chat_id = msg.chat.id.0,
            message_id = msg.id.0,
            "Failed to delete message",
        );
    }

    if db.get_last_list_message_id(msg.chat.id).await?.is_none() {
        let sent_msg = bot
            .send_message(msg.chat.id, NO_ACTIVE_LIST_TO_EDIT)
            .await?;
        crate::delete_after(bot.clone(), sent_msg.chat.id, sent_msg.id, 5);
        return Ok(());
    }

    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    if let Some(prev) = db.get_delete_session(user.id.0 as i64).await? {
        if let Some((c, m)) = prev.notice {
            if let Err(err) = bot.delete_message(c, m).await {
                tracing::warn!(
                    error = %err,
                    chat_id = c.0,
                    message_id = m.0,
                    "Failed to delete message",
                );
            }
        }
        if let Some(dm) = prev.dm_message_id {
            if let Err(err) = bot.delete_message(ChatId(user.id.0 as i64), dm).await {
                tracing::warn!(
                    error = %err,
                    chat_id = user.id.0,
                    message_id = dm.0,
                    "Failed to delete message",
                );
            }
        }
    }

    let items = db.list_items(msg.chat.id).await?;
    if items.is_empty() {
        return Ok(());
    }

    db.init_delete_session(user.id.0 as i64, msg.chat.id)
        .await?;

    let (base_text, keyboard) = { format_delete_list(&items, &HashSet::new()) };

    let chat_name = msg
        .chat
        .title()
        .map(ToString::to_string)
        .unwrap_or_else(|| DEFAULT_CHAT_NAME.to_string());
    let dm_text = delete_dm_text(&chat_name, &base_text);

    match bot
        .send_message(UserId(user.id.0), dm_text.clone())
        .reply_markup(keyboard)
        .await
    {
        Ok(dm_msg) => {
            db.set_delete_dm_message(user.id.0 as i64, dm_msg.id)
                .await?;
            if !msg.chat.is_private() {
                let info = bot
                    .send_message(msg.chat.id, delete_user_selecting_text(&user.first_name))
                    .await?;
                db.set_delete_notice(user.id.0 as i64, msg.chat.id, info.id)
                    .await?;
            }
        }
        Err(err) => {
            tracing::warn!("failed to send DM: {}", err);
            let warn = bot.send_message(msg.chat.id, DELETE_DM_FAILED).await?;
            crate::delete_after(bot.clone(), warn.chat.id, warn.id, 5);
        }
    }

    Ok(())
}

pub async fn callback_handler(bot: Bot, q: CallbackQuery, db: Database) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Some(id_str) = data.strip_prefix("delete_") {
            let user_id = q.from.id.0 as i64;

            if id_str == "done" {
                if let Some(session) = db.get_delete_session(user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
                        return Ok(());
                    }
                    for id in &session.selected {
                        db.delete_item(*id).await?;
                    }

                    if let Some(main_list_id) = db.get_last_list_message_id(session.chat_id).await?
                    {
                        update_list_message(&bot, session.chat_id, MessageId(main_list_id), &db)
                            .await?;
                    }

                    if let Some((chat_id, notice_id)) = session.notice {
                        if let Err(err) = bot.delete_message(chat_id, notice_id).await {
                            tracing::warn!(
                                error = %err,
                                chat_id = chat_id.0,
                                message_id = notice_id.0,
                                "Failed to delete message",
                            );
                        }
                    }

                    db.clear_delete_session(user_id).await?;
                }

                if let Err(err) = bot.delete_message(msg.chat().id, msg.id()).await {
                    tracing::warn!(
                        error = %err,
                        chat_id = msg.chat().id.0,
                        message_id = msg.id().0,
                        "Failed to delete message",
                    );
                }
            } else if let Ok(id) = id_str.parse::<i64>() {
                if let Some(mut session) = db.get_delete_session(user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
                        return Ok(());
                    }
                    if session.selected.contains(&id) {
                        session.selected.remove(&id);
                    } else {
                        session.selected.insert(id);
                    }
                    db.update_delete_selection(user_id, &session.selected)
                        .await?;
                    let items = db.list_items(session.chat_id).await?;
                    let (text, keyboard) = format_delete_list(&items, &session.selected);
                    if let Err(err) = bot
                        .edit_message_text(msg.chat().id, msg.id(), text)
                        .reply_markup(keyboard)
                        .await
                    {
                        tracing::warn!(
                            error = %err,
                            chat_id = msg.chat().id.0,
                            message_id = msg.id().0,
                            "Failed to edit message",
                        );
                    }
                }
            }
        } else if let Ok(id) = data.parse::<i64>() {
            db.toggle_item(id).await?;
            update_list_message(&bot, msg.chat().id, msg.id(), &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
