use anyhow::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::db::{Database, TokenRecord};
use crate::messages::{
    TOKENS_EMPTY, TOKEN_ISSUED, TOKEN_NOT_FOUND, TOKEN_REVOKED, TOKEN_REVOKE_USAGE,
};

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_preview(token: &str) -> String {
    token.chars().take(6).collect()
}

fn format_timestamp(timestamp: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_optional_timestamp(timestamp: Option<i64>, fallback: &str) -> String {
    timestamp
        .and_then(|value| chrono::DateTime::<chrono::Utc>::from_timestamp(value, 0))
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| fallback.to_string())
}

fn format_token_list(tokens: &[TokenRecord]) -> String {
    let mut lines = Vec::new();
    for token in tokens {
        let issued = format_timestamp(token.issued_at);
        let last_used = format_optional_timestamp(token.last_used_at, "never");
        let revoked = format_optional_timestamp(token.revoked_at, "not revoked");
        lines.push(format!(
            "<code>{}</code>\nissued: {issued}\nlast used: {last_used}\nrevoked: {revoked}",
            token.token
        ));
    }
    format!("<b>Tokens</b>\n\n{}", lines.join("\n\n"))
}

pub async fn issue_token(bot: Bot, msg: Message, db: Database) -> Result<()> {
    let token = generate_token();
    let issued_at = now_timestamp();
    let preview = token_preview(&token);
    tracing::debug!(
        chat_id = msg.chat.id.0,
        token_preview = %preview,
        "Issuing token"
    );
    db.create_token(msg.chat.id, &token, issued_at).await?;

    let response = format!("{TOKEN_ISSUED}\n<code>{token}</code>");
    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn list_tokens(bot: Bot, msg: Message, db: Database) -> Result<()> {
    let tokens = db.list_tokens(msg.chat.id).await?;
    if tokens.is_empty() {
        bot.send_message(msg.chat.id, TOKENS_EMPTY).await?;
        return Ok(());
    }

    let response = format_token_list(&tokens);
    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn revoke_token(bot: Bot, msg: Message, db: Database, token: String) -> Result<()> {
    let token = token.trim();
    if token.is_empty() {
        bot.send_message(msg.chat.id, TOKEN_REVOKE_USAGE).await?;
        return Ok(());
    }

    let revoked_at = now_timestamp();
    let preview = token_preview(token);
    tracing::debug!(
        chat_id = msg.chat.id.0,
        token_preview = %preview,
        "Revoking token"
    );
    let revoked = db.revoke_token(msg.chat.id, token, revoked_at).await?;
    let response = if revoked {
        TOKEN_REVOKED
    } else {
        TOKEN_NOT_FOUND
    };
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}
