use crate::db::Database;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::parse_items_gpt;
use crate::ai::stt::parse_items;
use crate::messages::{GPT_PARSING_DISABLED, HELP_TEXT};
use crate::text_utils::parse_item_line;

use super::list::{insert_capitalized_items_with_log, insert_items_with_log};

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
    );

    let _added =
        insert_capitalized_items_with_log(bot, msg.chat.id, &db, items, "via /parse").await?;

    Ok(())
}

fn parse_items_with_fallback(text: &str, gpt_result: anyhow::Result<Vec<String>>) -> Vec<String> {
    match gpt_result {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!(error = %err, "GPT parsing failed");
            parse_items(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_items_with_fallback;

    #[test]
    fn parse_items_with_fallback_uses_gpt_success() {
        let result = parse_items_with_fallback("ignored", Ok(vec!["Item".to_string()]));
        assert_eq!(result, vec!["Item".to_string()]);
    }

    #[test]
    fn parse_items_with_fallback_uses_local_parser_on_error() {
        let result = parse_items_with_fallback("a, b and c", Err(anyhow::anyhow!("nope")));
        assert_eq!(
            result,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }
}
