use crate::db::Database;
use crate::utils::download_file;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::vision::parse_photo_items;
use crate::text_utils::capitalize_first;

use super::list::insert_items;
use crate::ai::config::AiConfig;

pub async fn add_items_from_photo(
    bot: Bot,
    msg: Message,
    db: Database,
    ai_config: Option<AiConfig>,
) -> Result<()> {
    let Some(config) = ai_config else {
        return Ok(());
    };

    let photo_sizes = match msg.photo() {
        Some(p) => p,
        None => return Ok(()),
    };
    let Some(file_id) = photo_sizes
        .iter()
        .max_by_key(|p| p.file.size)
        .map(|p| &p.file.id)
    else {
        tracing::debug!("photo had no usable sizes");
        return Ok(());
    };

    let file = bot.get_file(file_id).await?;
    let bytes = download_file(&bot, &file.path).await?;
    tracing::trace!(size = bytes.len(), "downloaded photo bytes");

    tracing::debug!(model = %config.vision_model, "parsing photo with OpenAI vision");
    let items = match parse_photo_items(&config.api_key, &config.vision_model, &bytes, None).await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("photo parsing failed: {}", err);
            Vec::new()
        }
    };

    let items: Vec<String> = items.into_iter().map(|i| capitalize_first(&i)).collect();
    let added = insert_items(bot, msg.chat.id, &db, items).await?;
    if added > 0 {
        tracing::info!(
            "Added {} item(s) from photo for chat {}",
            added,
            msg.chat.id
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;

    #[tokio::test]
    async fn photo_with_no_sizes_returns_ok() {
        let db = init_test_db().await;
        let bot = Bot::new("test");
        let json = r#"{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"photo":[]}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        let ai_config = Some(AiConfig {
            api_key: "k".into(),
            stt_model: "m".into(),
            gpt_model: "g".into(),
            vision_model: "v".into(),
        });

        let res = add_items_from_photo(bot, msg, db, ai_config).await;
        assert!(res.is_ok());
    }
}
