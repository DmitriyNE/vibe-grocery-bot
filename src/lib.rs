use anyhow::Result;
use teloxide::prelude::*;

pub mod ai;
mod commands;
mod config;
pub mod db;
mod handlers;
mod messages;
mod system_info;
mod text_utils;
mod utils;

pub mod tests;

pub use ai::gpt::parse_items_gpt;
pub use ai::stt::parse_items;
pub use commands::Command;
pub use config::Config;
pub use db::Item;
pub use handlers::{
    add_items_from_parsed_text, add_items_from_photo, add_items_from_text, add_items_from_voice,
    ai_mode, callback_handler, enter_delete_mode, format_delete_list, format_list,
    format_plain_list, help, insert_items, show_system_info, ListService,
};
pub use messages::*;
pub use system_info::get_system_info;
pub use text_utils::{capitalize_first, normalize_for_match, parse_item_line};
pub use utils::delete_after;

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
    let delete_after_timeout = config.delete_after_timeout;
    let detector_model = config.detector_model.clone();

    // --- SQLite Pool ---
    let db_url = db::prepare_sqlite_url(&config.db_url);

    tracing::info!("Connecting to database at: {}", &db_url);

    let pool = db::connect_db(&db_url, config.db_pool_size).await?;
    let db = db::Database::new(pool);

    tracing::info!("Database connection successful.");

    sqlx::migrate!("./migrations").run(&*db).await?;

    // --- Command Enum ---
    // defined in the commands module

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
                     db: db::Database,
                     ai_config: Option<crate::ai::config::AiConfig>,
                     delete_after_timeout: u64,
                     detector_model: Option<String>| async move {
                        let service = ListService::new(&db);
                        match cmd {
                            Command::Start | Command::Help => help(bot, msg).await?,
                            Command::List => service.send_list(bot, msg.chat.id).await?,
                            Command::Archive => service.archive(bot, msg.chat.id).await?,
                            Command::Delete => {
                                enter_delete_mode(bot, msg, &db, delete_after_timeout).await?
                            }
                            Command::Share => service.share_list(bot, msg.chat.id).await?,
                            Command::Nuke => service.nuke(bot, msg, delete_after_timeout).await?,
                            Command::Parse => {
                                add_items_from_parsed_text(bot, msg, db, ai_config).await?
                            }
                            Command::Info => show_system_info(bot, msg).await?,
                            Command::AiMode => ai_mode(bot, msg, detector_model.clone()).await?,
                        }
                        Ok(())
                    },
                ))
                .branch(dptree::endpoint(add_items_from_text)),
        );

    // --- Dispatcher ---
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            db,
            ai_config,
            delete_after_timeout,
            detector_model
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
