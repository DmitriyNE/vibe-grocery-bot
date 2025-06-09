use shopbot::insert_items;
use shopbot::tests::util::init_test_db;
use teloxide::prelude::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn insert_items_adds_and_sends() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"ok":true,"result":{"message_id":1,"date":0,"chat":{"id":1,"type":"private"}}}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;

    let added = insert_items(bot, ChatId(1), &db, vec!["Milk".to_string()])
        .await
        .unwrap();
    assert_eq!(added, 1);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
    server.verify().await;
}

#[tokio::test]
async fn insert_items_empty_sends_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let db = init_test_db().await;

    let added = insert_items(bot, ChatId(1), &db, Vec::<String>::new())
        .await
        .unwrap();
    assert_eq!(added, 0);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    server.verify().await;
}
