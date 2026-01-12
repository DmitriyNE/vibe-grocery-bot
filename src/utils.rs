use anyhow::Result;
use futures_util::StreamExt;
use teloxide::{
    net::Download,
    prelude::*,
    types::{ChatId, InlineKeyboardMarkup, MessageId},
    RequestError,
};

/// Default timeout in seconds for temporary messages.
pub const DEFAULT_DELETE_AFTER_TIMEOUT: u64 = 5;

/// Delete a message after the given delay in seconds.
pub fn delete_after(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    secs: u64,
) -> tokio::task::JoinHandle<()> {
    tracing::debug!(
        chat_id = chat_id.0,
        message_id = message_id.0,
        delay_secs = secs,
        "Scheduling message deletion"
    );
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        let res = bot.delete_message(chat_id, message_id).await;
        if let Err(ref err) = res {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = message_id.0,
                "Failed to delete message",
            );
        }
        tracing::debug!(
            chat_id = chat_id.0,
            message_id = message_id.0,
            "Finished delete_after task"
        );
    })
}

/// Attempt to delete a message and log a warning on failure.
pub async fn try_delete_message(bot: &Bot, chat_id: ChatId, message_id: MessageId) {
    if let Err(err) = bot.delete_message(chat_id, message_id).await {
        tracing::warn!(
            error = %err,
            chat_id = chat_id.0,
            message_id = message_id.0,
            "Failed to delete message",
        );
    }
}

/// Attempt to edit a message and log a warning on failure.
pub async fn try_edit_message(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    text: impl Into<String>,
    markup: InlineKeyboardMarkup,
) {
    if let Err(err) = bot
        .edit_message_text(chat_id, message_id, text)
        .reply_markup(markup)
        .await
    {
        tracing::warn!(
            error = %err,
            chat_id = chat_id.0,
            message_id = message_id.0,
            "Failed to edit message",
        );
    }
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

/// Fetch a Telegram file by its `file_id` and return the raw bytes.
pub async fn download_telegram_file(bot: &Bot, file_id: &str) -> Result<Vec<u8>> {
    let file = bot.get_file(file_id).await?;
    tracing::debug!(path = %file.path, "Downloading Telegram file");
    let bytes = download_file(bot, &file.path).await?;
    tracing::debug!(path = %file.path, size = bytes.len(), "Finished download");
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use reqwest::Client;
    use teloxide::{types::InlineKeyboardButton, RequestError};
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn try_delete_message_sends_request() -> Result<(), RequestError> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/botTEST/[Dd]eleteMessage$"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::builder().no_proxy().build().unwrap();
        let bot = Bot::with_client("TEST", client)
            .set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
        try_delete_message(&bot, ChatId(1), MessageId(2)).await;
        server.verify().await;
        Ok(())
    }

    #[tokio::test]
    async fn try_edit_message_sends_request() -> Result<(), RequestError> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/botTEST/EditMessageText"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::builder().no_proxy().build().unwrap();
        let bot = Bot::with_client("TEST", client)
            .set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
        let markup = InlineKeyboardMarkup::new(Vec::<Vec<InlineKeyboardButton>>::new());
        try_edit_message(&bot, ChatId(1), MessageId(2), "hi", markup).await;
        server.verify().await;
        Ok(())
    }

    #[tokio::test]
    async fn download_telegram_file_gets_bytes() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/botTEST/GetFile"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"ok":true,"result":{"file_id":"f","file_unique_id":"u","file_path":"path"}}"#,
                "application/json",
            ))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/file/botTEST/path"))
            .respond_with(ResponseTemplate::new(200).set_body_raw("hi", "application/octet-stream"))
            .expect(1)
            .mount(&server)
            .await;

        let url = reqwest::Url::parse(&server.uri()).unwrap();
        let client = Client::builder().no_proxy().build().unwrap();
        let bot = Bot::with_client("TEST", client).set_api_url(url);
        let bytes = download_telegram_file(&bot, "f").await?;
        assert_eq!(bytes, b"hi");
        server.verify().await;
        Ok(())
    }
}
