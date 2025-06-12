use anyhow::Result;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tracing::{debug, instrument, trace};

pub use crate::ai::prompts::DEFAULT_STT_PROMPT as DEFAULT_PROMPT;

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

const OPENAI_STT_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

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
    let builder = client.post(url).multipart(form);
    let resp = crate::ai::common::send_openai_request(api_key, builder).await?;

    let raw = resp.text().await?;
    let snippet: String = raw.chars().take(200).collect();
    debug!(snippet = %snippet, "transcription response body");
    let data: TranscriptionResponse = serde_json::from_str(&raw)?;
    trace!(transcription = %data.text, "transcription successful");
    Ok(data.text)
}

#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn transcribe_audio(
    model: &str,
    api_key: &str,
    prompt: Option<&str>,
    bytes: &[u8],
    url: Option<&str>,
) -> Result<String> {
    let url = url.unwrap_or(OPENAI_STT_URL);
    transcribe_audio_inner(model, api_key, prompt, bytes, url).await
}

/// Split a text string into individual items.
///
/// The input is split on commas (`","`), newline characters, and the literal
/// word `"and"`. Each resulting segment is passed through
/// [`crate::text_utils::parse_item_line`], which trims whitespace and removes
/// status markers. Empty segments are dropped because `parse_item_line` returns
/// `None` when nothing remains after cleaning.
pub fn parse_items(text: &str) -> Vec<String> {
    text.split([',', '\n'])
        .flat_map(|seg| seg.split(" and "))
        .filter_map(crate::text_utils::parse_item_line)
        .collect()
}
