use shopbot::tests::util::init_test_db;
use teloxide::{prelude::*, utils::command::BotCommands};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "test commands")]
enum Command {
    Start,
    Help,
    List,
    Archive,
    Delete,
    Share,
    Nuke,
    Parse,
    Info,
}

#[tokio::test]
async fn dispatcher_add_then_list() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"ok":true,"result":{"message_id":1,"date":0,"chat":{"id":1,"type":"private"}}}"#,
            "application/json",
        ))
        .expect(3)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let ai_config: Option<shopbot::ai::config::AiConfig> = None;

    let handler = dptree::entry()
        .branch(Update::filter_callback_query().endpoint(shopbot::callback_handler))
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.voice().is_some())
                        .endpoint(shopbot::add_items_from_voice),
                )
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.photo().is_some())
                        .endpoint(shopbot::add_items_from_photo),
                )
                .branch(dptree::entry().filter_command::<Command>().endpoint(
                    |bot: Bot,
                     msg: Message,
                     cmd: Command,
                     db: shopbot::db::Database,
                     ai_config: Option<shopbot::ai::config::AiConfig>| async move {
                        match cmd {
                            Command::Start | Command::Help => shopbot::help(bot, msg).await?,
                            Command::List => shopbot::send_list(bot, msg.chat.id, &db).await?,
                            Command::Archive => shopbot::archive(bot, msg.chat.id, &db).await?,
                            Command::Delete => shopbot::enter_delete_mode(bot, msg, &db).await?,
                            Command::Share => shopbot::share_list(bot, msg.chat.id, &db).await?,
                            Command::Nuke => shopbot::nuke_list(bot, msg, &db).await?,
                            Command::Parse => {
                                shopbot::add_items_from_parsed_text(bot, msg, db, ai_config).await?
                            }
                            Command::Info => shopbot::show_system_info(bot, msg).await?,
                        }
                        Ok(())
                    },
                ))
                .branch(dptree::endpoint(shopbot::add_items_from_text)),
        );

    let add_update: Update = serde_json::from_str(
        r#"{"update_id":1,"message":{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"text":"Milk"}}"#,
    )
    .unwrap();
    let list_update: Update = serde_json::from_str(
        r#"{"update_id":2,"message":{"message_id":2,"date":0,"chat":{"id":1,"type":"private"},"text":"/list","entities":[{"type":"bot_command","offset":0,"length":5}]}}"#,
    )
    .unwrap();

    let me = teloxide::types::Me {
        user: teloxide::types::User {
            id: teloxide::types::UserId(1),
            is_bot: true,
            first_name: "Test".into(),
            last_name: None,
            username: Some("testbot".into()),
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        },
        can_join_groups: true,
        can_read_all_group_messages: true,
        supports_inline_queries: false,
        can_connect_to_business: false,
    };
    let _ = handler
        .dispatch(dptree::deps![
            add_update,
            bot.clone(),
            me.clone(),
            db.clone(),
            ai_config.clone()
        ])
        .await;
    let _ = handler
        .dispatch(dptree::deps![list_update, bot, me, db, ai_config])
        .await;

    server.verify().await;
}
