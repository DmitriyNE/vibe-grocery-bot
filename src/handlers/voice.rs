use crate::utils::download_file;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::{interpret_voice_command, VoiceCommand};
use crate::ai::stt::{parse_voice_items, transcribe_audio, DEFAULT_PROMPT};
use crate::db::{add_item, delete_item, list_items};
use crate::text_utils::{capitalize_first, normalize_for_match};

use crate::db::Item;

pub async fn delete_matching_items(
    db: &Pool<Sqlite>,
    current: &mut Vec<Item>,
    items: &[String],
) -> Result<Vec<String>> {
    let mut deleted = Vec::new();
    for item in items {
        let needle = normalize_for_match(item);
        if let Some(pos) = current
            .iter()
            .position(|i| normalize_for_match(&i.text) == needle)
        {
            let found = current.remove(pos);
            delete_item(db, found.id).await?;
            deleted.push(found.text);
        }
    }
    Ok(deleted)
}

use super::list::send_list;

pub async fn add_items_from_voice(
    bot: Bot,
    msg: Message,
    db: Pool<Sqlite>,
    ai_config: Option<AiConfig>,
) -> Result<()> {
    let Some(config) = ai_config else {
        return Ok(());
    };

    let voice = match msg.voice() {
        Some(v) => v,
        None => return Ok(()),
    };

    let file = bot.get_file(&voice.file.id).await?;
    let audio = download_file(&bot, &file.path).await?;

    match transcribe_audio(
        &config.stt_model,
        &config.api_key,
        Some(DEFAULT_PROMPT),
        &audio,
        None,
    )
    .await
    {
        Ok(text) => {
            if text.trim().is_empty() {
                tracing::debug!("voice transcription empty; ignoring");
                return Ok(());
            }
            let mut current = list_items(&db, msg.chat.id).await?;
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
                    let deleted = delete_matching_items(&db, &mut current, &items).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;
    use teloxide::types::ChatId;

    #[tokio::test]
    async fn delete_matching_multiple() {
        let db = init_test_db().await;
        let chat = ChatId(1);
        for _ in 0..3 {
            add_item(&db, chat, "Item").await.unwrap();
        }

        let mut current = list_items(&db, chat).await.unwrap();
        let deleted = delete_matching_items(
            &db,
            &mut current,
            &["Item".to_string(), "Item".to_string(), "Item".to_string()],
        )
        .await
        .unwrap();
        assert_eq!(deleted.len(), 3);
        let remaining = list_items(&db, chat).await.unwrap();
        assert!(remaining.is_empty());
    }
}
