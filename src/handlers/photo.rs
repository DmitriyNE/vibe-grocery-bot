use anyhow::Result;
use futures_util::StreamExt;
use sqlx::{Pool, Sqlite};
use teloxide::{net::Download, prelude::*};

use crate::ai::vision::parse_photo_items;
use crate::db::add_item;
use crate::text_utils::capitalize_first;

use super::list::send_list;
use crate::ai::config::AiConfig;

pub async fn add_items_from_photo(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    stt: Option<AiConfig>,
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

    let items = match parse_photo_items(&config.api_key, &config.vision_model, &bytes).await {
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
