use crate::ai::common::{request_items, OPENAI_CHAT_URL};
use anyhow::Result;
use tracing::instrument;

/// Use the OpenAI Chat API to parse items from arbitrary text.
///
/// The model is instructed to return a JSON object with an `items` array. The
/// returned list is cleaned with [`crate::text_utils::parse_item_line`].
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
                "content": "Extract the items from the user's text. Preserve numbers exactly as provided, such as '1 milk'. Respond with a JSON object like {\"items\": [\"1 milk\"]}.",
            },
            { "role": "user", "content": text },
        ]
    });

    request_items(api_key, &body, url).await
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
