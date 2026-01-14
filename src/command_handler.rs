use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::commands::Command;
use crate::db;

pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    db: db::Database,
    ai_config: Option<AiConfig>,
    delete_after_timeout: u64,
) -> Result<()> {
    cmd.dispatch(bot, msg, db, ai_config, delete_after_timeout)
        .await
}
