use shopbot::db::ChatKey;
use shopbot::tests::util::init_test_db;
use shopbot::{archive, nuke_list, LIST_ARCHIVED, LIST_NUKED};
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
            format!(r#"{{"ok":true,"result":{{"message_id":1,"date":0,"chat":{{"id":1,"type":"private"}},"text":"{}"}}}}"#, LIST_ARCHIVED),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    let key = ChatKey(chat.0);
    db.add_item(key, "Milk").await.unwrap();
    db.update_last_list_message_id(key, MessageId(10))
        .await
        .unwrap();

    archive(bot, chat, &db).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&*db)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert!(db.get_last_list_message_id(key).await.unwrap().is_none());

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

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;
    let chat = ChatId(1);
    let key = ChatKey(chat.0);
    db.add_item(key, "Milk").await.unwrap();
    db.update_last_list_message_id(key, MessageId(5))
        .await
        .unwrap();

    let msg: Message = serde_json::from_str(
        r#"{"message_id":1,"date":0,"chat":{"id":1,"type":"private"},"text":"/nuke"}"#,
    )
    .unwrap();

    nuke_list(bot, msg, &db, 5).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&*db)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert!(db.get_last_list_message_id(key).await.unwrap().is_none());

    server.verify().await;
}
