use crate::db::Database;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::parse_items_gpt;
use crate::messages::{GPT_PARSING_DISABLED, HELP_TEXT};
use crate::text_utils::parse_item_line;

use super::list::{insert_capitalized_items_with_log, insert_items_with_log};
use super::parse::parse_items_with_fallback;

pub async fn help(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(msg.chat.id, HELP_TEXT)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn add_items_from_text(bot: Bot, msg: Message, db: Database) -> Result<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };
    let items: Vec<String> = text.lines().filter_map(parse_item_line).collect();

    let _added = insert_items_with_log(bot, msg.chat.id, &db, items, "via text message").await?;
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

    let items = parse_items_with_fallback(
        text,
        parse_items_gpt(
            &config.api_key,
            &config.gpt_model,
            text,
            config.openai_chat_url.as_deref(),
        )
        .await,
        "gpt_parse",
    );

    let _added =
        insert_capitalized_items_with_log(bot, msg.chat.id, &db, items, "via /parse").await?;

    Ok(())
}
