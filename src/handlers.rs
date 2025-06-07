use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId},
};

use crate::db::*;

pub async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "Send me any text to add it to your shopping list. Each line will be a new item.\n\
         You can tap the checkbox button next to an item to mark it as bought.\n\n\
         <b>Commands:</b>\n\
         /list - Show the current shopping list.\n\
         /archive - Finalize and archive the current list, starting a new one.\n\
         /delete - Show a temporary panel to delete items from the list.\n\
         /share - Send the list as plain text for copying.\n\
         /nuke - Completely delete the current list.\n\
         /parse - Parse this message into items via GPT.",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}

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

pub fn format_delete_list(
    items: &[Item],
    selected: &HashSet<i64>,
) -> (String, InlineKeyboardMarkup) {
    let text = "Select items to delete, then tap 'Done Deleting'.".to_string();

    let mut keyboard_buttons = Vec::new();

    for item in items {
        let button_text = if selected.contains(&item.id) {
            format!("🗑️ {}", item.text)
        } else {
            format!("❌ {}", item.text)
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

pub fn format_plain_list(items: &[Item]) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&format!("• {}\n", item.text));
    }
    text
}

/// Clean a single text line from a user message.
///
/// Returns `None` if the line should be ignored (for example it is the
/// archived list separator or becomes empty after trimming). Otherwise returns
/// the cleaned line without leading status emojis or whitespace.
pub fn parse_item_line(line: &str) -> Option<String> {
    tracing::trace!(?line, "Parsing item line");
    if line.trim() == "--- Archived List ---" {
        tracing::trace!("Ignoring archived list separator");
        return None;
    }

    let cleaned = line
        .trim_start_matches(['☑', '✅', '⬜', '🛒', '\u{fe0f}'])
        .trim();

    if cleaned.is_empty() {
        tracing::trace!("Line empty after cleaning");
        None
    } else {
        let result = cleaned.to_string();
        tracing::trace!(?result, "Parsed line");
        Some(result)
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

use crate::ai::stt::{
    parse_items, parse_items_gpt, parse_voice_items, parse_voice_items_gpt, transcribe_audio,
    SttConfig, DEFAULT_PROMPT,
};
use crate::ai::vision::parse_photo_items;
use futures_util::StreamExt;
use teloxide::net::Download;

pub async fn add_items_from_voice(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    stt: Option<SttConfig>,
) -> Result<()> {
    let Some(config) = stt else {
        return Ok(());
    };

    let voice = match msg.voice() {
        Some(v) => v,
        None => return Ok(()),
    };

    let file = bot.get_file(&voice.file.id).await?;
    let mut audio = Vec::new();
    let mut stream = bot.download_file_stream(&file.path);
    while let Some(chunk) = stream.next().await {
        audio.extend_from_slice(&chunk?);
    }

    match transcribe_audio(&config.model, &config.api_key, Some(DEFAULT_PROMPT), &audio).await {
        Ok(text) => {
            let items = match parse_voice_items_gpt(&config.api_key, &text).await {
                Ok(list) => list,
                Err(err) => {
                    tracing::warn!("gpt parsing failed: {}", err);
                    parse_voice_items(&text)
                }
            };
            let mut added = 0;
            for item in items {
                let cap = capitalize_first(&item);
                add_item(&db, msg.chat.id, &cap).await?;
                added += 1;
            }
            if added > 0 {
                tracing::info!(
                    "Added {} item(s) from voice for chat {}",
                    added,
                    msg.chat.id
                );
                send_list(bot, msg.chat.id, &db).await?;
            }
        }
        Err(err) => {
            tracing::warn!("transcription failed: {}", err);
        }
    }

    Ok(())
}

pub async fn add_items_from_photo(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    stt: Option<SttConfig>,
) -> Result<()> {
    let Some(config) = stt else {
        return Ok(());
    };

    let photo_sizes = match msg.photo() {
        Some(p) => p,
        None => return Ok(()),
    };
    let file_id = photo_sizes
        .iter()
        .max_by_key(|p| p.file.size)
        .map(|p| &p.file.id)
        .unwrap();

    let file = bot.get_file(file_id).await?;
    let mut bytes = Vec::new();
    let mut stream = bot.download_file_stream(&file.path);
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
    }

    let items = match parse_photo_items(&config.api_key, &bytes).await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("photo parsing failed: {}", err);
            Vec::new()
        }
    };

    let mut added = 0;
    for item in items {
        let cap = capitalize_first(&item);
        add_item(&db, msg.chat.id, &cap).await?;
        added += 1;
    }

    if added > 0 {
        tracing::info!(
            "Added {} item(s) from photo for chat {}",
            added,
            msg.chat.id
        );
        send_list(bot, msg.chat.id, &db).await?;
    }

    Ok(())
}

pub async fn add_items_from_text(bot: Bot, msg: Message, db: Pool<Sqlite>) -> Result<()> {
    if let Some(text) = msg.text() {
        let mut items_added_count = 0;
        for line in text.lines() {
            if let Some(cleaned_line) = parse_item_line(line) {
                add_item(&db, msg.chat.id, &cleaned_line).await?;
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

pub async fn add_items_from_parsed_text(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    stt: Option<SttConfig>,
) -> Result<()> {
    let Some(config) = stt else {
        bot.send_message(msg.chat.id, "GPT parsing is disabled.")
            .await?;
        return Ok(());
    };

    let Some(text) = msg.text() else {
        return Ok(());
    };

    let items = match parse_items_gpt(&config.api_key, text).await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("gpt parsing failed: {}", err);
            parse_items(text)
        }
    };

    let mut added = 0;
    for item in items {
        let cap = capitalize_first(&item);
        add_item(&db, msg.chat.id, &cap).await?;
        added += 1;
    }

    if added > 0 {
        tracing::info!(
            "Added {} item(s) via /parse for chat {}",
            added,
            msg.chat.id
        );
        send_list(bot, msg.chat.id, &db).await?;
    }

    Ok(())
}

pub async fn send_list(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Sending list");
    if let Some(message_id) = get_last_list_message_id(db, chat_id).await? {
        let _ = bot.delete_message(chat_id, MessageId(message_id)).await;
    }

    let items = list_items(db, chat_id).await?;
    tracing::trace!(
        chat_id = chat_id.0,
        items_count = items.len(),
        "Fetched items for list"
    );

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

pub async fn share_list(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Sharing list");
    let items = list_items(db, chat_id).await?;
    if items.is_empty() {
        bot.send_message(chat_id, "Your shopping list is empty!")
            .await?;
        return Ok(());
    }

    let text = format_plain_list(&items);
    bot.send_message(chat_id, text).await?;

    Ok(())
}

pub async fn update_list_message(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    db: &Pool<Sqlite>,
) -> Result<()> {
    tracing::debug!(
        chat_id = chat_id.0,
        message_id = message_id.0,
        "Updating list message"
    );
    let items = list_items(db, chat_id).await?;
    tracing::trace!(items_count = items.len(), "Fetched items for update");

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

pub async fn archive(bot: Bot, chat_id: ChatId, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(chat_id = chat_id.0, "Archiving list");
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

pub async fn enter_delete_mode(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(
        chat_id = msg.chat.id.0,
        user_id = msg.from().map(|u| u.id.0),
        "Entering delete mode"
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

    let user = match msg.from() {
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

pub async fn nuke_list(bot: Bot, msg: Message, db: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!(chat_id = msg.chat.id.0, "Nuking list");
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

pub async fn callback_handler(bot: Bot, q: CallbackQuery, db: Pool<Sqlite>) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Some(id_str) = data.strip_prefix("delete_") {
            let user_id = q.from.id.0 as i64;

            if id_str == "done" {
                if let Some(session) = get_delete_session(&db, user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id.0) {
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

                let _ = bot.delete_message(msg.chat.id, msg.id).await;
            } else if let Ok(id) = id_str.parse::<i64>() {
                if let Some(mut session) = get_delete_session(&db, user_id).await? {
                    if session.dm_message_id.map(|m| m.0) != Some(msg.id.0) {
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
