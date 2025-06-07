use anyhow::Result;
use futures_util::StreamExt;
use sqlx::{Pool, Sqlite};
use teloxide::{net::Download, prelude::*};

use crate::ai::gpt::parse_voice_items_gpt;
use crate::ai::stt::{parse_voice_items, transcribe_audio, SttConfig, DEFAULT_PROMPT};
use crate::db::add_item;
use crate::text_utils::capitalize_first;

use super::list::send_list;

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
