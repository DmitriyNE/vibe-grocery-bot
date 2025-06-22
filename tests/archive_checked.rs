use shopbot::tests::util::init_test_db;
use shopbot::{ListService, NO_CHECKED_ITEMS_TO_ARCHIVE};
use teloxide::{prelude::*, types::MessageId};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn archive_checked_archives_only_done() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/botTEST/EditMessageText"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/botTEST/SendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"ok":true,"result":{"message_id":42,"date":0,"chat":{"id":1,"type":"private"},"text":"t"}}"#,
            "application/json",
        ))
        .expect(2)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    db.add_item(chat, "Milk").await.unwrap();
    db.add_item(chat, "Eggs").await.unwrap();
    let items = db.list_items(chat).await.unwrap();
    db.toggle_item(chat, items[0].id).await.unwrap();
    db.update_last_list_message_id(chat, MessageId(5))
        .await
        .unwrap();

    ListService::new(&db)
        .archive_checked(bot, chat)
        .await
        .unwrap();

    let remaining = db.list_items(chat).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].text, "Eggs");
    server.verify().await;
}

#[tokio::test]
async fn archive_checked_none_done() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/botTEST/SendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(r#"{{"ok":true,"result":{{"message_id":1,"date":0,"chat":{{"id":1,"type":"private"}},"text":"{}"}}}}"#, NO_CHECKED_ITEMS_TO_ARCHIVE),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    db.add_item(chat, "Milk").await.unwrap();
    db.update_last_list_message_id(chat, MessageId(3))
        .await
        .unwrap();

    ListService::new(&db)
        .archive_checked(bot, chat)
        .await
        .unwrap();

    let remaining = db.list_items(chat).await.unwrap();
    assert_eq!(remaining.len(), 1);
    server.verify().await;
}
