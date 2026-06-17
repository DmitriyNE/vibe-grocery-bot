use reqwest::Client;
use shopbot::tests::util::init_test_db;
use shopbot::{ListService, LIST_NUKED};
use teloxide::{
    prelude::*,
    types::{Message, MessageId},
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn archive_clears_data_and_sends_confirmation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/botTEST/DeleteMessage"))
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
            r#"{"ok":true,"result":{"message_id":2,"date":0,"chat":{"id":1,"type":"private"},"text":"archived"}}"#,
            "application/json",
        ))
        .expect(2)
        .mount(&server)
        .await;

    let client = Client::builder().no_proxy().build().unwrap();
    let bot =
        Bot::with_client("TEST", client).set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    db.add_item_count(chat, "Milk").await.unwrap();
    db.update_last_list_message_id(chat, MessageId(10))
        .await
        .unwrap();

    ListService::new(&db).archive(bot, chat).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&*db)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert!(db.get_last_list_message_id(chat).await.unwrap().is_none());

    server.verify().await;
}

#[tokio::test]
async fn nuke_clears_data_and_sends_confirmation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/botTEST/DeleteMessage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
        )
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/botTEST/SendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(r#"{{"ok":true,"result":{{"message_id":2,"date":0,"chat":{{"id":1,"type":"private"}},"text":"{}"}}}}"#, LIST_NUKED),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder().no_proxy().build().unwrap();
    let bot =
        Bot::with_client("TEST", client).set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    db.add_item_count(chat, "Milk").await.unwrap();
    db.update_last_list_message_id(chat, MessageId(5))
        .await
        .unwrap();

    let msg: Message = serde_json::from_str(
        r#"{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"text":"/nuke"}"#,
    )
    .unwrap();

    ListService::new(&db).nuke(bot, msg, 5).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&*db)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert!(db.get_last_list_message_id(chat).await.unwrap().is_none());

    server.verify().await;
}
