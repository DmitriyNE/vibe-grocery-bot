use crate::utils::download_file;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::ai::gpt::{interpret_voice_command, VoiceCommand};
use crate::ai::stt::{parse_voice_items, transcribe_audio, DEFAULT_PROMPT};
#[cfg(test)]
use crate::db::add_item;
use crate::db::{delete_items, list_items};
use crate::text_utils::{capitalize_first, normalize_for_match};

use crate::db::Item;

pub async fn delete_matching_items(
    db: &Pool<Sqlite>,
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
    delete_items(db, &ids).await?;
    Ok(deleted)
}

use super::list::{insert_items, send_list};

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
                    let items: Vec<String> =
                        items.into_iter().map(|i| capitalize_first(&i)).collect();
                    let added = insert_items(bot.clone(), msg.chat.id, &db, items).await?;
                    if added > 0 {
                        tracing::info!(
                            "Added {} item(s) from voice for chat {}",
                            added,
                            msg.chat.id
                        );
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
                    let items: Vec<String> =
                        items.into_iter().map(|i| capitalize_first(&i)).collect();
                    let added = insert_items(bot.clone(), msg.chat.id, &db, items).await?;
                    if added > 0 {
                        tracing::info!(
                            "Added {} item(s) from voice for chat {}",
                            added,
                            msg.chat.id
                        );
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
        assert!(current.is_empty());
        let remaining = list_items(&db, chat).await.unwrap();
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn delete_matching_partial() {
        let db = init_test_db().await;
        let chat = ChatId(1);
        add_item(&db, chat, "Apple").await.unwrap();
        add_item(&db, chat, "Banana").await.unwrap();
        add_item(&db, chat, "Carrot").await.unwrap();

        let mut current = list_items(&db, chat).await.unwrap();
        let deleted = delete_matching_items(
            &db,
            &mut current,
            &["Banana".to_string(), "Carrot".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(deleted, vec!["Banana".to_string(), "Carrot".to_string()]);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].text, "Apple");

        let remaining = list_items(&db, chat).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].text, "Apple");
    }
}
