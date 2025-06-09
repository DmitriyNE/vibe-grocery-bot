use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId},
};

use crate::db::*;

use super::list::update_list_message;

pub fn format_delete_list(
    items: &[Item],
    selected: &HashSet<i64>,
) -> (String, InlineKeyboardMarkup) {
    let text = "Select items to delete, then tap 'Done Deleting'.".to_string();

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
        "🗑️ Done Deleting",
        "delete_done",
    )]);

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

pub async fn enter_delete_mode(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(
        chat_id = msg.chat.id.0,
        user_id = msg.from.as_ref().map(|u| u.id.0),
        "Entering delete mode",
    );
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

    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    if let Some(prev) = get_delete_session(db, user.id.0 as i64).await? {
        if let Some((c, m)) = prev.notice {
            let _ = bot.delete_message(c, m).await;
        }
        if let Some(dm) = prev.dm_message_id {
            let _ = bot.delete_message(ChatId(user.id.0 as i64), dm).await;
        }
    }

    let items = list_items(db, msg.chat.id).await?;
    if items.is_empty() {
        return Ok(());
    }

    init_delete_session(db, user.id.0 as i64, msg.chat.id).await?;

    let (base_text, keyboard) = { format_delete_list(&items, &HashSet::new()) };

    let chat_name = msg
        .chat
        .title()
        .map(ToString::to_string)
        .unwrap_or_else(|| "your list".to_string());
    let dm_text = format!("Deleting items from {}.\n\n{}", chat_name, base_text);

    match bot
        .send_message(UserId(user.id.0), dm_text.clone())
        .reply_markup(keyboard)
        .await
    {
        Ok(dm_msg) => {
            set_delete_dm_message(db, user.id.0 as i64, dm_msg.id).await?;
            if !msg.chat.is_private() {
                let info = bot
                    .send_message(
                        msg.chat.id,
                        format!("{} is selecting items to delete...", user.first_name),
                    )
                    .await?;
                set_delete_notice(db, user.id.0 as i64, msg.chat.id, info.id).await?;
            }
        }
        Err(err) => {
            tracing::warn!("failed to send DM: {}", err);
            let warn = bot
                .send_message(
                    msg.chat.id,
                    "Unable to send you a private delete panel. Have you started me in private?",
                )
                .await?;
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                let _ = bot.delete_message(warn.chat.id, warn.id).await;
            });
        }
    }

    Ok(())
}

pub async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Some(id_str) = data.strip_prefix("delete_") {
            let user_id = q.from.id.0 as i64;

            if id_str == "done" {
                if let Some(session) = get_delete_session(&db, user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
                        return Ok(());
                    }
                    for id in &session.selected {
                        delete_item(&db, *id).await?;
                    }

                    if let Some(main_list_id) =
                        get_last_list_message_id(&db, session.chat_id).await?
                    {
                        update_list_message(&bot, session.chat_id, MessageId(main_list_id), &db)
                            .await?;
                    }

                    if let Some((chat_id, notice_id)) = session.notice {
                        let _ = bot.delete_message(chat_id, notice_id).await;
                    }

                    clear_delete_session(&db, user_id).await?;
                }

                let _ = bot.delete_message(msg.chat().id, msg.id()).await;
            } else if let Ok(id) = id_str.parse::<i64>() {
                if let Some(mut session) = get_delete_session(&db, user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
                        return Ok(());
                    }
                    if session.selected.contains(&id) {
                        session.selected.remove(&id);
                    } else {
                        session.selected.insert(id);
                    }
                    update_delete_selection(&db, user_id, &session.selected).await?;
                    let items = list_items(&db, session.chat_id).await?;
                    let (text, keyboard) = format_delete_list(&items, &session.selected);
                    let _ = bot
                        .edit_message_text(msg.chat().id, msg.id(), text)
                        .reply_markup(keyboard)
                        .await;
                }
            }
        } else if let Ok(id) = data.parse::<i64>() {
            toggle_item(&db, id).await?;
            update_list_message(&bot, msg.chat().id, msg.id(), &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
