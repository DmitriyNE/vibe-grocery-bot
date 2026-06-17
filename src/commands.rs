use anyhow::Result;
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::ai::config::AiConfig;
use crate::db;
use crate::handlers::{
    add_items_from_parsed_text, enter_delete_mode, help, issue_token, list_tokens, revoke_token,
    show_system_info, ListService,
};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "show the current list.")]
    List,
    #[command(description = "finalize and archive the current list, starting a new one.")]
    Archive,
    #[command(
        rename = "done",
        description = "archive only checked items and keep the rest."
    )]
    ArchiveDone,
    #[command(description = "show a temporary panel to delete items from the list.")]
    Delete,
    #[command(description = "send the list as plain text for copying.")]
    Share,
    #[command(description = "completely delete the current list.")]
    Nuke,
    #[command(description = "parse items from the given text using GPT.")]
    Parse,
    #[command(description = "show system information.")]
    Info,
    #[command(
        rename = "create_token",
        description = "issue a new token for this list (optionally named)."
    )]
    CreateToken(String),
    #[command(description = "list issued tokens for this list.")]
    Tokens,
    #[command(rename = "revoke_token", description = "revoke a token.")]
    RevokeToken(String),
}

impl Command {
    pub async fn dispatch(
        self,
        bot: Bot,
        msg: Message,
        db: db::Database,
        ai_config: Option<AiConfig>,
        delete_after_timeout: u64,
    ) -> Result<()> {
        let service = ListService::new(&db);
        match self {
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
}
