use anyhow::Result;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tracing::{debug, instrument, trace};

/// Default instructions passed to GPT-based transcription models.
/// The prompt also asks the model to keep verbs intact so commands like
/// "delete" are not dropped during transcription. Quantities should be
/// written using digits when possible. Convert spelled-out numbers to digits
/// so phrases like "три ананаса" become "3 ананаса".
pub const DEFAULT_PROMPT: &str = "Transcribe the user's request about the list. Keep verbs like 'add' or 'delete' exactly as spoken. Use digits for quantities and convert number words to digits.";

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
    let builder = client.post(url).multipart(form);
    let resp = crate::ai::common::send_openai_request(api_key, builder).await?;

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
    url: Option<&str>,
) -> Result<String> {
    let url = url.unwrap_or(OPENAI_URL);
    transcribe_audio_inner(model, api_key, prompt, bytes, url).await
}

/// Split a text string into individual items.
///
/// The input is split on commas, newlines and the word "and". Each segment is
/// cleaned via [`crate::text_utils::parse_item_line`]. Empty segments are
/// ignored.
pub fn parse_items(text: &str) -> Vec<String> {
    text.split([',', '\n'])
        .flat_map(|seg| seg.split(" and "))
        .filter_map(crate::text_utils::parse_item_line)
        .collect()
}
