use anyhow::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::TryRngCore;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, User};
use teloxide::utils::html::escape;

use crate::db::{Database, TokenRecord};
use crate::messages::{
    TOKENS_EMPTY, TOKEN_ISSUED, TOKEN_NOT_FOUND, TOKEN_REVOKED, TOKEN_REVOKE_USAGE,
};

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .expect("OS RNG should be available to issue tokens");
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
        let name = token
            .name
            .as_deref()
            .map(|value| format!("name: {}\n", escape(value)))
            .unwrap_or_default();
        let issuer = match (token.issuer_name.as_deref(), token.issuer_user_id) {
            (Some(name), Some(user_id)) => {
                format!("issued by: {} ({user_id})\n", escape(name))
            }
            (Some(name), None) => format!("issued by: {}\n", escape(name)),
            (None, Some(user_id)) => format!("issued by: {user_id}\n"),
            (None, None) => String::new(),
        };
        lines.push(format!(
            "<code>{}</code>\n{name}{issuer}issued: {issued}\nlast used: {last_used}\nrevoked: {revoked}",
            token.token,
        ));
    }
    format!("<b>Tokens</b>\n\n{}", lines.join("\n\n"))
}

fn parse_token_name(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn issuer_display_name(user: &User) -> String {
    if let Some(username) = user.username.as_deref() {
        format!("@{username}")
    } else if let Some(last_name) = user.last_name.as_deref() {
        format!("{} {}", user.first_name, last_name)
    } else {
        user.first_name.clone()
    }
}

pub async fn issue_token(
    bot: Bot,
    msg: Message,
    db: Database,
    requested_name: String,
) -> Result<()> {
    let token = generate_token();
    let issued_at = now_timestamp();
    let preview = token_preview(&token);
    let name = parse_token_name(&requested_name);
    let issuer_user_id = msg.from.as_ref().map(|user| user.id.0 as i64);
    let issuer_name = msg.from.as_ref().map(issuer_display_name);
    tracing::debug!(
        chat_id = msg.chat.id.0,
        token_preview = %preview,
        name = name.as_deref(),
        issuer_user_id,
        issuer_name = issuer_name.as_deref(),
        "Issuing token"
    );
    db.create_token(
        msg.chat.id,
        &token,
        name.as_deref(),
        issuer_user_id,
        issuer_name.as_deref(),
        issued_at,
    )
    .await?;

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
