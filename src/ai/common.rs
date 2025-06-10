use anyhow::{anyhow, Result};
use serde::Deserialize;
use tracing::{debug, instrument, trace, warn};

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ItemsJson {
    items: Vec<String>,
}

pub const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Build a chat completion request body for text input.
pub fn build_text_chat_body(
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_text },
        ]
    })
}

/// Build a chat completion request body for an image input.
pub fn build_image_chat_body(
    model: &str,
    system_prompt: &str,
    image_url: &str,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system_prompt },
            {
                "role": "user",
                "content": [ { "type": "image_url", "image_url": { "url": image_url } } ],
            }
        ]
    })
}

#[instrument(level = "trace", skip(api_key, builder))]
pub async fn send_openai_request(
    api_key: &str,
    builder: reqwest::RequestBuilder,
) -> Result<reqwest::Response> {
    let resp = builder.bearer_auth(api_key).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        warn!(%status, "OpenAI API error");
        return Err(anyhow!("OpenAI API error {status}: {err_text}"));
    }

    Ok(resp)
}

#[instrument(level = "trace", skip(api_key, body))]
pub async fn request_items(
    api_key: &str,
    body: &serde_json::Value,
    url: &str,
) -> Result<Vec<String>> {
    debug!(url, "sending chat completion request");

    let client = reqwest::Client::new();
    let builder = client.post(url).json(body);
    let resp = send_openai_request(api_key, builder).await?;

    let raw = resp.text().await?;
    trace!(raw = %raw, "chat response");
    let chat: ChatResponse = serde_json::from_str(&raw)?;
    let content = chat
        .choices
        .first()
        .ok_or_else(|| anyhow!("missing chat choice"))?
        .message
        .content
        .trim()
        .to_string();

    let items_json: ItemsJson = serde_json::from_str(&content)?;

    Ok(items_json
        .items
        .into_iter()
        .filter_map(|s| crate::text_utils::parse_item_line(&s))
        .collect())
}
