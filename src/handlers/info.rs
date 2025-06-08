use anyhow::Result;
use teloxide::prelude::*;

use crate::system_info::get_system_info;

pub async fn show_system_info(bot: Bot, msg: Message) -> Result<()> {
    tracing::debug!(chat_id = msg.chat.id.0, "Showing system info");
    bot.send_message(msg.chat.id, get_system_info()).await?;
    Ok(())
}
