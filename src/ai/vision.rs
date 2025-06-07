use crate::ai::common::{request_items, OPENAI_CHAT_URL};
use anyhow::Result;
use base64::Engine as _;
use tracing::instrument;

#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn parse_photo_items(api_key: &str, bytes: &[u8]) -> Result<Vec<String>> {
    parse_photo_items_inner(api_key, bytes, OPENAI_CHAT_URL).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn parse_photo_items_inner(
    api_key: &str,
    bytes: &[u8],
    url: &str,
) -> Result<Vec<String>> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:image/png;base64,{}", encoded);
    let body = serde_json::json!({
        "model": "gpt-4o",
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Extract the items shown in the photo. Respond with a JSON object like {\"items\": [\"apples\"]}.",
            },
            {
                "role": "user",
                "content": [ { "type": "image_url", "image_url": { "url": data_url } } ],
            }
        ]
    });

    request_items(api_key, &body, url).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn parse_photo_items_test(api_key: &str, bytes: &[u8], url: &str) -> Result<Vec<String>> {
    parse_photo_items_inner(api_key, bytes, url).await
}
