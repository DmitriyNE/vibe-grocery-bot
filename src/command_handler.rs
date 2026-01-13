use anyhow::Result;
use teloxide::prelude::*;

use crate::ai::config::AiConfig;
use crate::commands::Command;
use crate::db;
use crate::handlers::{
    add_items_from_parsed_text, enter_delete_mode, help, issue_token, list_tokens, revoke_token,
    show_system_info, ListService,
};

pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    db: db::Database,
    ai_config: Option<AiConfig>,
    delete_after_timeout: u64,
) -> Result<()> {
    let service = ListService::new(&db);
    match cmd {
        Command::Start | Command::Help => help(bot, msg).await?,
        Command::List => service.send_list(bot, msg.chat.id).await?,
        Command::Archive => service.archive(bot, msg.chat.id).await?,
        Command::ArchiveDone => service.archive_checked(bot, msg.chat.id).await?,
        Command::Delete => enter_delete_mode(bot, msg, &db, delete_after_timeout).await?,
        Command::Share => service.share_list(bot, msg.chat.id).await?,
        Command::Nuke => service.nuke(bot, msg, delete_after_timeout).await?,
        Command::Parse => add_items_from_parsed_text(bot, msg, db, ai_config).await?,
        Command::Info => show_system_info(bot, msg).await?,
        Command::CreateToken(name) => issue_token(bot, msg, db, name).await?,
        Command::Tokens => list_tokens(bot, msg, db).await?,
        Command::RevokeToken(token) => revoke_token(bot, msg, db, token).await?,
    }
    Ok(())
}
