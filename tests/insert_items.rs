use shopbot::insert_items;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use teloxide::prelude::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn init_test_db() -> Pool<Sqlite> {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory database");

    sqlx::query(
        "CREATE TABLE items(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    text TEXT NOT NULL,\n    done BOOLEAN NOT NULL DEFAULT 0\n)",
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE chat_state(\n    chat_id INTEGER PRIMARY KEY,\n    last_list_message_id INTEGER\n)",
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE delete_session(\n    user_id INTEGER PRIMARY KEY,\n    chat_id INTEGER NOT NULL,\n    selected TEXT NOT NULL DEFAULT '',\n    notice_chat_id INTEGER,\n    notice_message_id INTEGER,\n    dm_message_id INTEGER\n)",
    )
    .execute(&db)
    .await
    .unwrap();

    db
}

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
