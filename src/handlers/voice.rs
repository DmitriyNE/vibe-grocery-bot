use anyhow::Result;
use futures_util::StreamExt;
use sqlx::{Pool, Sqlite};
use teloxide::{net::Download, prelude::*};

use crate::ai::gpt::{interpret_voice_command, VoiceCommand};
use crate::ai::stt::{parse_voice_items, transcribe_audio, SttConfig, DEFAULT_PROMPT};
use crate::db::{add_item, delete_item, list_items};
use crate::text_utils::{capitalize_first, normalize_for_match};

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
            if text.trim().is_empty() {
                tracing::debug!("voice transcription empty; ignoring");
                return Ok(());
            }
            let current = list_items(&db, msg.chat.id).await?;
            let list_texts: Vec<String> = current.iter().map(|i| i.text.clone()).collect();
            match interpret_voice_command(&config.api_key, &config.gpt_model, &text, &list_texts)
                .await
            {
                Ok(VoiceCommand::Add(items)) => {
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
                        send_list(bot.clone(), msg.chat.id, &db).await?;
                    }
                }
                Ok(VoiceCommand::Delete(items)) => {
                    let mut deleted = Vec::new();
                    for item in items {
                        let needle = normalize_for_match(&item);
                        if let Some(found) = current
                            .iter()
                            .find(|i| normalize_for_match(&i.text) == needle)
                        {
                            delete_item(&db, found.id).await?;
                            deleted.push(found.text.clone());
                        }
                    }
                    if !deleted.is_empty() {
                        tracing::info!(
                            "Deleted {} item(s) via voice for chat {}",
                            deleted.len(),
                            msg.chat.id
                        );
                        let lines: Vec<String> = deleted.iter().map(|t| format!("• {t}")).collect();
                        let msg_text =
                            format!("🗑 Removed via voice request:\n{}", lines.join("\n"));
                        bot.send_message(msg.chat.id, msg_text).await?;
                        send_list(bot.clone(), msg.chat.id, &db).await?;
                    }
                }
                Err(err) => {
                    tracing::warn!("gpt command failed: {}", err);
                    let items = parse_voice_items(&text);
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
                        send_list(bot.clone(), msg.chat.id, &db).await?;
                    }
                }
            }
        }
        Err(err) => {
            tracing::warn!("transcription failed: {}", err);
        }
    }

    Ok(())
}
