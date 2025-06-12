use crate::ai::common::{build_items_request, request_items, OPENAI_CHAT_URL};
use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

/// Use the OpenAI Chat API to parse items from arbitrary text.
///
/// The model is instructed to return a JSON object with an `items` array. The
/// returned list is cleaned with [`crate::text_utils::parse_item_line`].
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_items_gpt(
    api_key: &str,
    model: &str,
    text: &str,
    url: Option<&str>,
) -> Result<Vec<String>> {
    let url = url.unwrap_or(OPENAI_CHAT_URL);
    parse_items_gpt_inner(api_key, model, text, url).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key))]
pub async fn parse_items_gpt_inner(
    api_key: &str,
    model: &str,
    text: &str,
    url: &str,
) -> Result<Vec<String>> {
    let prompt = "Extract the items from the user's text. Use the nominative form for nouns when it does not change the meaning. Convert number words to digits so 'три ананаса' becomes '3 ананаса'. Respond with a JSON object like {\"items\": [\"1 milk\"]}";
    let body = build_items_request(model, prompt, Value::String(text.to_string()));

    request_items(api_key, &body, url).await
}

#[derive(Debug, PartialEq)]
pub enum VoiceCommand {
    Add(Vec<String>),
    Delete(Vec<String>),
}

#[derive(serde::Deserialize)]
struct CommandJson {
    add: Option<Vec<String>>,
    delete: Option<Vec<String>>,
}

#[instrument(level = "trace", skip(api_key))]
pub async fn interpret_voice_command(
    api_key: &str,
    model: &str,
    text: &str,
    list: &[String],
    url: Option<&str>,
) -> Result<VoiceCommand> {
    let url = url.unwrap_or(OPENAI_CHAT_URL);
    interpret_voice_command_inner(api_key, model, text, list, url).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key))]
pub async fn interpret_voice_command_inner(
    api_key: &str,
    model: &str,
    text: &str,
    list: &[String],
    url: &str,
) -> Result<VoiceCommand> {
    let list_text = if list.is_empty() {
        "The list is empty.".to_string()
    } else {
        format!("Current items: {}.", list.join(", "))
    };
    let list_json = serde_json::to_string(list)?;

    let prompt = format!(
        "You manage a list of items. {list_text} The list as JSON is {list_json}. Decide whether the user's request adds items or removes items from the list. Return a JSON object like {{\"add\":[...]}} or {{\"delete\":[...]}}. For deletions, include each item exactly as it appears in the list, including any leading quantities. If unsure, treat it as an addition request. Use nominative forms for item names when possible and convert number words to digits."
    );

    let body = crate::ai::common::build_text_chat_body(model, &prompt, text);

    use tracing::{debug, trace};

    debug!(url, "sending chat completion request");

    let client = reqwest::Client::new();
    let builder = client.post(url).json(&body);
    let resp = crate::ai::common::send_openai_request(api_key, builder).await?;

    let raw = resp.text().await?;
    let snippet: String = raw.chars().take(200).collect();
    debug!(snippet = %snippet, "chat response body");
    trace!(raw = %raw, "chat response");
    let content = crate::ai::common::parse_chat_content(&raw)?;

    let cmd: CommandJson = serde_json::from_str(&content)?;

    if let Some(delete) = cmd.delete {
        let cleaned: Vec<String> = delete
            .into_iter()
            .filter_map(|s| crate::text_utils::parse_item_line(&s))
            .collect();
        Ok(VoiceCommand::Delete(cleaned))
    } else {
        let add = cmd.add.unwrap_or_default();
        let cleaned: Vec<String> = add
            .into_iter()
            .filter_map(|s| crate::text_utils::parse_item_line(&s))
            .collect();
        Ok(VoiceCommand::Add(cleaned))
    }
}

#[cfg(test)]
#[instrument(level = "trace", skip(api_key))]
pub async fn interpret_voice_command_test(
    api_key: &str,
    model: &str,
    text: &str,
    list: &[String],
    url: &str,
) -> Result<VoiceCommand> {
    interpret_voice_command_inner(api_key, model, text, list, url).await
}

// Re-export the inner implementation so integration tests can still call
// `interpret_voice_command_test` even though the real helper is gated off.
#[cfg(not(test))]
pub use interpret_voice_command_inner as interpret_voice_command_test;
