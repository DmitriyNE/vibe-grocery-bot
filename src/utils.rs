use futures_util::StreamExt;
use teloxide::{
    net::Download,
    prelude::*,
    types::{ChatId, MessageId},
    RequestError,
};

/// Delete a message after the given delay in seconds.
pub fn delete_after(bot: Bot, chat_id: ChatId, message_id: MessageId, secs: u64) {
    tracing::debug!(
        chat_id = chat_id.0,
        message_id = message_id.0,
        delay_secs = secs,
        "Scheduling message deletion"
    );
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        if let Err(err) = bot.delete_message(chat_id, message_id).await {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = message_id.0,
                "Failed to delete message",
            );
        }
    });
}

/// Download a file from Telegram and return the raw bytes.
pub async fn download_file(bot: &Bot, path: &str) -> Result<Vec<u8>, RequestError> {
    let mut data = Vec::new();
    let mut stream = bot.download_file_stream(path);
    while let Some(chunk) = stream.next().await {
        data.extend_from_slice(&chunk?);
    }
    tracing::trace!(size = data.len(), "downloaded file bytes");
    Ok(data)
}
