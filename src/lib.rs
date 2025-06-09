use anyhow::Result;
use sqlx::{Pool, Sqlite};
use teloxide::{prelude::*, utils::command::BotCommands};

pub mod ai;
mod config;
mod db;
mod handlers;
mod system_info;
mod text_utils;
mod utils;

pub mod tests;

pub use ai::gpt::parse_items_gpt;
pub use ai::stt::parse_items;
pub use config::Config;
pub use db::Item;
pub use handlers::{format_delete_list, format_list, format_plain_list, insert_items};
pub use system_info::get_system_info;
pub use text_utils::{capitalize_first, normalize_for_match, parse_item_line};
pub use utils::delete_after;

use handlers::{
    add_items_from_parsed_text, add_items_from_photo, add_items_from_text, add_items_from_voice,
    archive, callback_handler, enter_delete_mode, help, nuke_list, send_list, share_list,
    show_system_info,
};

pub async fn run() -> Result<()> {
    let config = Config::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting list bot...");

    let bot = Bot::from_env();

    if let Some(cfg) = &config.ai {
        tracing::debug!(
            stt_model = cfg.stt_model,
            gpt_model = cfg.gpt_model,
            vision_model = cfg.vision_model,
            "OpenAI configuration loaded"
        );
    }
    let ai_config = config.ai.clone();

    // --- SQLite Pool ---
    let db_url = db::prepare_sqlite_url(&config.db_url);

    tracing::info!("Connecting to database at: {}", &db_url);

    let db = db::connect_db(&db_url).await?;

    tracing::info!("Database connection successful.");

    sqlx::migrate!("./migrations").run(&db).await?;

    // --- Command Enum ---
    #[derive(BotCommands, Clone)]
    #[command(
        rename_rule = "lowercase",
        description = "These commands are supported:"
    )]
    enum Command {
        #[command(description = "display this text.")]
        Start,
        #[command(description = "display this text.")]
        Help,
        #[command(description = "show the current shopping list.")]
        List,
        #[command(description = "finalize and archive the current list, starting a new one.")]
        Archive,
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
    }

    // --- Handler Setup ---
    let handler = dptree::entry()
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.voice().is_some())
                        .endpoint(add_items_from_voice),
                )
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.photo().is_some())
                        .endpoint(add_items_from_photo),
                )
                .branch(dptree::entry().filter_command::<Command>().endpoint(
                    |bot: Bot,
                     msg: Message,
                     cmd: Command,
                     db: Pool<Sqlite>,
                     ai_config: Option<crate::ai::config::AiConfig>| async move {
                        match cmd {
                            Command::Start | Command::Help => help(bot, msg).await?,
                            Command::List => send_list(bot, msg.chat.id, &db).await?,
                            Command::Archive => archive(bot, msg.chat.id, &db).await?,
                            Command::Delete => enter_delete_mode(bot, msg, &db).await?,
                            Command::Share => share_list(bot, msg.chat.id, &db).await?,
                            Command::Nuke => nuke_list(bot, msg, &db).await?,
                            Command::Parse => {
                                add_items_from_parsed_text(bot, msg, db, ai_config).await?
                            }
                            Command::Info => show_system_info(bot, msg).await?,
                        }
                        Ok(())
                    },
                ))
                .branch(dptree::endpoint(add_items_from_text)),
        );

    // --- Dispatcher ---
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db, ai_config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[cfg(test)]
mod lib_tests {
    use super::*;
    use crate::db::*;
    use crate::tests::util::init_test_db;
    use teloxide::types::MessageId;

    #[tokio::test]
    async fn basic_item_flow() -> Result<()> {
        let db = init_test_db().await;
        let chat = ChatId(42);

        add_item(&db, chat, "Apples").await?;
        add_item(&db, chat, "Milk").await?;

        let mut items = list_items(&db, chat).await?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "Apples");
        assert!(!items[0].done);

        toggle_item(&db, items[0].id).await?;
        items = list_items(&db, chat).await?;
        assert!(items[0].done);

        delete_item(&db, items[1].id).await?;
        items = list_items(&db, chat).await?;
        assert_eq!(items.len(), 1);

        delete_all_items(&db, chat).await?;
        items = list_items(&db, chat).await?;
        assert!(items.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn last_message_id_roundtrip() -> Result<()> {
        let db = init_test_db().await;
        let chat = ChatId(1);

        assert!(get_last_list_message_id(&db, chat).await?.is_none());

        update_last_list_message_id(&db, chat, MessageId(99)).await?;
        assert_eq!(get_last_list_message_id(&db, chat).await?, Some(99));

        clear_last_list_message_id(&db, chat).await?;
        assert!(get_last_list_message_id(&db, chat).await?.is_none());

        Ok(())
    }
}
