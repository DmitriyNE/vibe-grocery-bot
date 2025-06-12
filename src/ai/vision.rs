use crate::ai::common::{build_items_request, request_items, OPENAI_CHAT_URL};
use anyhow::Result;
use base64::Engine as _;
use serde_json::json;
use tracing::instrument;

#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn parse_photo_items(
    api_key: &str,
    model: &str,
    bytes: &[u8],
    url: Option<&str>,
) -> Result<Vec<String>> {
    let url = url.unwrap_or(OPENAI_CHAT_URL);
    parse_photo_items_inner(api_key, model, bytes, url).await
}

#[cfg_attr(not(test), allow(dead_code))]
#[instrument(level = "trace", skip(api_key, bytes))]
pub async fn parse_photo_items_inner(
    api_key: &str,
    model: &str,
    bytes: &[u8],
    url: &str,
) -> Result<Vec<String>> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:image/png;base64,{}", encoded);
    let prompt = "Extract the items shown in the photo. Respond with a JSON object like {\"items\": [\"apples\"]}.";
    let body = build_items_request(
        model,
        prompt,
        json!([{ "type": "image_url", "image_url": { "url": data_url } }]),
    );

    request_items(api_key, &body, url).await
}
