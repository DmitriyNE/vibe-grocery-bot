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

/// Extract the first choice's message content from a chat completion response.
pub fn parse_chat_content(raw: &str) -> Result<String> {
    let chat: ChatResponse = serde_json::from_str(raw)?;
    let content = chat
        .choices
        .first()
        .ok_or_else(|| anyhow!("missing chat choice"))?
        .message
        .content
        .trim()
        .to_string();
    Ok(content)
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
    build_items_request(
        model,
        system_prompt,
        serde_json::Value::String(user_text.to_string()),
    )
}

/// Build a chat completion request body for an image input.
pub fn build_image_chat_body(
    model: &str,
    system_prompt: &str,
    image_url: &str,
) -> serde_json::Value {
    build_items_request(
        model,
        system_prompt,
        serde_json::json!([{
            "type": "image_url",
            "image_url": { "url": image_url },
        }]),
    )
}

/// Build a chat completion request body for item extraction.
pub fn build_items_request(
    model: &str,
    prompt: &str,
    user_payload: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": prompt },
            { "role": "user", "content": user_payload },
        ]
    })
}

#[instrument(level = "trace", skip(api_key, builder))]
pub async fn send_openai_request(
    api_key: &str,
    builder: reqwest::RequestBuilder,
) -> Result<reqwest::Response> {
    let url = builder
        .try_clone()
        .and_then(|b| b.build().ok())
        .map(|req| req.url().clone());

    let resp = builder.bearer_auth(api_key).send().await?;
    debug!(url = %url.as_ref().map(|u| u.as_str()).unwrap_or(""), status = %resp.status(), "OpenAI request completed");

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        let snippet: String = err_text.chars().take(200).collect();
        debug!(url = %url.as_ref().map(|u| u.as_str()).unwrap_or(""), %status, snippet = %snippet, "error body");
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
    let snippet: String = raw.chars().take(200).collect();
    debug!(snippet = %snippet, "chat response body");
    trace!(raw = %raw, "chat response");
    let content = parse_chat_content(&raw)?;

    let items_json: ItemsJson = serde_json::from_str(&content)?;

    Ok(items_json
        .items
        .into_iter()
        .filter_map(|s| crate::text_utils::parse_item_line(&s))
        .collect())
}
