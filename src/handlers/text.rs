use crate::db::Database;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::parse_items_gpt;
use crate::ai::stt::parse_items;
use crate::text_utils::{capitalize_first, parse_item_line};

use super::list::insert_items;

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

pub async fn add_items_from_text(bot: Bot, msg: Message, db: Database) -> Result<()> {
    if let Some(text) = msg.text() {
        let items: Vec<String> = text.lines().filter_map(parse_item_line).collect();

        let added = insert_items(bot, msg.chat.id, &db, items).await?;
        if added > 0 {
            tracing::info!("Added {} item(s) for chat {}", added, msg.chat.id);
        }
    }
    Ok(())
}

pub async fn add_items_from_parsed_text(
    bot: Bot,
    msg: Message,
    db: Database,
    ai_config: Option<AiConfig>,
) -> Result<()> {
    let Some(config) = ai_config else {
        bot.send_message(msg.chat.id, "GPT parsing is disabled.")
            .await?;
        return Ok(());
    };

    let Some(text) = msg.text() else {
        return Ok(());
    };

    let items = match parse_items_gpt(&config.api_key, &config.gpt_model, text, None).await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("gpt parsing failed: {}", err);
            parse_items(text)
        }
    };

    let items: Vec<String> = items.into_iter().map(|i| capitalize_first(&i)).collect();
    let added = insert_items(bot, msg.chat.id, &db, items).await?;
    if added > 0 {
        tracing::info!(
            "Added {} item(s) via /parse for chat {}",
            added,
            msg.chat.id
        );
    }

    Ok(())
}
