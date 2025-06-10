use crate::db::Database;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::parse_items_gpt;
use crate::ai::stt::parse_items;
use crate::messages::{GPT_PARSING_DISABLED, HELP_TEXT};
use crate::text_utils::{capitalize_first, parse_item_line};

use super::list::insert_items;

pub async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(msg.chat.id, HELP_TEXT)
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
        bot.send_message(msg.chat.id, GPT_PARSING_DISABLED).await?;
        return Ok(());
    };

    let Some(text) = msg.text() else {
        return Ok(());
    };

    let items = match parse_items_gpt(
        &config.api_key,
        &config.gpt_model,
        text,
        config.openai_chat_url.as_deref(),
    )
    .await
    {
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
