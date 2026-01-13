use crate::db::Database;
use crate::utils::download_telegram_file;
use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::{interpret_voice_command, VoiceCommand};
use crate::ai::stt::{parse_items, transcribe_audio, DEFAULT_PROMPT};
use crate::messages::VOICE_REMOVED_PREFIX;
use crate::text_utils::normalize_for_match;

use crate::db::Item;

pub async fn delete_matching_items(
    db: &Database,
    chat_id: ChatId,
    current: &mut Vec<Item>,
    items: &[String],
) -> Result<Vec<String>> {
    let mut deleted = Vec::new();
    let mut ids = Vec::new();
    for item in items {
        let needle = normalize_for_match(item);
        if let Some(pos) = current
            .iter()
            .position(|i| normalize_for_match(&i.text) == needle)
        {
            let found = current.remove(pos);
            ids.push(found.id);
            deleted.push(found.text);
        }
    }
    db.delete_items(chat_id, &ids).await?;
    Ok(deleted)
}

use super::list::insert_capitalized_items_with_log;
use super::list_service::ListService;

pub async fn add_items_from_voice(
    bot: Bot,
    msg: Message,
    db: Database,
    ai_config: Option<AiConfig>,
) -> Result<()> {
    let Some(config) = ai_config else {
        return Ok(());
    };

    let voice = match msg.voice() {
        Some(v) => v,
        None => return Ok(()),
    };

    let audio = download_telegram_file(&bot, &voice.file.id).await?;

    match transcribe_audio(
        &config.stt_model,
        &config.api_key,
        Some(DEFAULT_PROMPT),
        &audio,
        config.openai_stt_url.as_deref(),
    )
    .await
    {
        Ok(text) => {
            if text.trim().is_empty() {
                tracing::debug!("voice transcription empty; ignoring");
                return Ok(());
            }
            let mut current = db.list_items(msg.chat.id).await?;
            let list_texts: Vec<String> = current.iter().map(|i| i.text.clone()).collect();
            match interpret_voice_command(
                &config.api_key,
                &config.gpt_model,
                &text,
                &list_texts,
                config.openai_chat_url.as_deref(),
            )
            .await
            {
                Ok(VoiceCommand::Add(items)) => {
                    let _added = insert_capitalized_items_with_log(
                        bot.clone(),
                        msg.chat.id,
                        &db,
                        items,
                        "from voice",
                    )
                    .await?;
                }
                Ok(VoiceCommand::Delete(items)) => {
                    let deleted =
                        delete_matching_items(&db, msg.chat.id, &mut current, &items).await?;
                    if !deleted.is_empty() {
                        tracing::info!(
                            "Deleted {} item(s) via voice for chat {}",
                            deleted.len(),
                            msg.chat.id
                        );
                        let lines: Vec<String> = deleted.iter().map(|t| format!("• {t}")).collect();
                        let msg_text = format!("{VOICE_REMOVED_PREFIX}{}", lines.join("\n"));
                        bot.send_message(msg.chat.id, msg_text).await?;
                        ListService::new(&db)
                            .send_list(bot.clone(), msg.chat.id)
                            .await?;
                    }
                }
                Err(err) => {
                    tracing::warn!("gpt command failed: {}", err);
                    let items = parse_items(&text);
                    let _added = insert_capitalized_items_with_log(
                        bot.clone(),
                        msg.chat.id,
                        &db,
                        items,
                        "from voice",
                    )
                    .await?;
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
            db.add_item(chat, "Item").await.unwrap();
        }

        let mut current = db.list_items(chat).await.unwrap();
        let deleted = delete_matching_items(
            &db,
            chat,
            &mut current,
            &["Item".to_string(), "Item".to_string(), "Item".to_string()],
        )
        .await
        .unwrap();
        assert_eq!(deleted.len(), 3);
        assert!(current.is_empty());
        let remaining = db.list_items(chat).await.unwrap();
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn delete_matching_partial() {
        let db = init_test_db().await;
        let chat = ChatId(1);
        db.add_item(chat, "Apple").await.unwrap();
        db.add_item(chat, "Banana").await.unwrap();
        db.add_item(chat, "Carrot").await.unwrap();

        let mut current = db.list_items(chat).await.unwrap();
        let deleted = delete_matching_items(
            &db,
            chat,
            &mut current,
            &["Banana".to_string(), "Carrot".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(deleted, vec!["Banana".to_string(), "Carrot".to_string()]);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].text, "Apple");

        let remaining = db.list_items(chat).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].text, "Apple");
    }
}
