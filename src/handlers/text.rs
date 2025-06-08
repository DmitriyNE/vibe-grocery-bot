use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::prelude::*;

use crate::ai::gpt::parse_items_gpt;
use crate::ai::stt::{parse_items, SttConfig};
use crate::db::add_item;
use crate::text_utils::{capitalize_first, parse_item_line};

use super::list::send_list;

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
             /parse - Parse this message into items via GPT.\n\
             /info - Show system information.",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
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
