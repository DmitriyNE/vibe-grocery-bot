use teloxide::{
    prelude::*,
    types::{ChatId, MessageId},
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
        let _ = bot.delete_message(chat_id, message_id).await;
    });
}
