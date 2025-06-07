use anyhow::{anyhow, Result};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tracing::{debug, instrument, trace, warn};

#[derive(Clone)]
pub struct SttConfig {
    pub api_key: String,
    pub model: String,
}

/// Default instructions passed to GPT-based transcription models.
pub const DEFAULT_PROMPT: &str = "List the items mentioned, separated by commas or the word 'and'.";

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

const OPENAI_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

#[instrument(level = "trace", skip(api_key, bytes))]
async fn transcribe_audio_inner(
    model: &str,
    api_key: &str,
    prompt: Option<&str>,
    bytes: &[u8],
    url: &str,
) -> Result<String> {
    let part = Part::bytes(bytes.to_vec()).file_name("voice.ogg");
    let mut form = Form::new()
        .part("file", part)
        .text("model", model.to_string());
    if let Some(p) = prompt {
        form = form.text("prompt", p.to_string());
    }

    debug!(model, prompt=?prompt, url, "sending transcription request");

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        warn!(%status, "OpenAI API error");
        return Err(anyhow!("OpenAI API error {status}: {err_text}"));
    }

    let data: TranscriptionResponse = resp.json().await?;
    trace!(transcription = %data.text, "transcription successful");
    Ok(data.text)
}

#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn transcribe_audio(
    model: &str,
    api_key: &str,
    prompt: Option<&str>,
    bytes: &[u8],
) -> Result<String> {
    transcribe_audio_inner(model, api_key, prompt, bytes, OPENAI_URL).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn transcribe_audio_test(
    model: &str,
    api_key: &str,
    prompt: Option<&str>,
    bytes: &[u8],
    url: &str,
) -> Result<String> {
    transcribe_audio_inner(model, api_key, prompt, bytes, url).await
}

/// Split a transcription string from speech-to-text into individual items.
///
/// The text is split on commas, newlines and the word "and". Each segment is
/// then cleaned via [`crate::handlers::parse_item_line`]. Empty segments are
/// ignored.
/// Split a text string into individual items.
///
/// The input is split on commas, newlines and the word "and". Each segment is
/// then cleaned via [`crate::handlers::parse_item_line`]. Empty segments are
/// ignored.
pub fn parse_items(text: &str) -> Vec<String> {
    text.split([',', '\n'])
        .flat_map(|seg| seg.split(" and "))
        .filter_map(crate::handlers::parse_item_line)
        .collect()
}

/// Legacy wrapper for [`parse_items`] used by older code paths.
pub fn parse_voice_items(text: &str) -> Vec<String> {
    parse_items(text)
}

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

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Use the OpenAI Chat API to parse items from arbitrary text.
///
/// The model is instructed to return a JSON object with an `items` array. The
/// returned list is cleaned with [`crate::handlers::parse_item_line`].
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_items_gpt(api_key: &str, text: &str) -> Result<Vec<String>> {
    parse_items_gpt_inner(api_key, text, OPENAI_CHAT_URL).await
}

/// Legacy wrapper for [`parse_items_gpt`] used by voice message handling.
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_voice_items_gpt(api_key: &str, text: &str) -> Result<Vec<String>> {
    parse_items_gpt(api_key, text).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_items_gpt_inner(api_key: &str, text: &str, url: &str) -> Result<Vec<String>> {
    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Extract the items from the user's text. Respond with a JSON object like {\"items\": [\"apples\"]}.",
            },
            { "role": "user", "content": text },
        ]
    });

    debug!(url, "sending chat completion request");

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        warn!(%status, "OpenAI API error");
        return Err(anyhow!("OpenAI API error {status}: {err_text}"));
    }

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
        .filter_map(|s| crate::handlers::parse_item_line(&s))
        .collect())
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_items_gpt_test(api_key: &str, text: &str, url: &str) -> Result<Vec<String>> {
    parse_items_gpt_inner(api_key, text, url).await
}

/// Legacy wrapper for [`parse_items_gpt_test`].
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_voice_items_gpt_test(
    api_key: &str,
    text: &str,
    url: &str,
) -> Result<Vec<String>> {
    parse_items_gpt_test(api_key, text, url).await
}
