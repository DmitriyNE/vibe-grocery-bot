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
/// then cleaned via [`crate::text_utils::parse_item_line`]. Empty segments are
/// ignored.
/// Split a text string into individual items.
///
/// The input is split on commas, newlines and the word "and". Each segment is
/// then cleaned via [`crate::text_utils::parse_item_line`]. Empty segments are
/// ignored.
pub fn parse_items(text: &str) -> Vec<String> {
    text.split([',', '\n'])
        .flat_map(|seg| seg.split(" and "))
        .filter_map(crate::text_utils::parse_item_line)
        .collect()
}

/// Legacy wrapper for [`parse_items`] used by older code paths.
pub fn parse_voice_items(text: &str) -> Vec<String> {
    parse_items(text)
}
