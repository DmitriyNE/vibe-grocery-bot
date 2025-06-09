use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

pub async fn init_test_db() -> Pool<Sqlite> {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory database");

    sqlx::query(
        "CREATE TABLE items(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    text TEXT NOT NULL,\n    done BOOLEAN NOT NULL DEFAULT 0\n)"
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE chat_state(\n    chat_id INTEGER PRIMARY KEY,\n    last_list_message_id INTEGER\n)"
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE delete_session(\n    user_id INTEGER PRIMARY KEY,\n    chat_id INTEGER NOT NULL,\n    selected TEXT NOT NULL DEFAULT '',\n    notice_chat_id INTEGER,\n    notice_message_id INTEGER,\n    dm_message_id INTEGER\n)"
    )
    .execute(&db)
    .await
    .unwrap();

    db
}
