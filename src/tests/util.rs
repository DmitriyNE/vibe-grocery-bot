use crate::db::{connect_db, Database};

pub async fn init_test_db() -> Database {
    let pool = connect_db("sqlite::memory:", 1)
        .await
        .expect("failed to create in-memory database");

    sqlx::query(
        "CREATE TABLE items(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    text TEXT NOT NULL,\n    done BOOLEAN NOT NULL DEFAULT 0\n)"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE chat_state(\n    chat_id INTEGER PRIMARY KEY,\n    last_list_message_id INTEGER\n)"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE delete_session(\n    user_id INTEGER PRIMARY KEY,\n    chat_id INTEGER NOT NULL,\n    selected TEXT NOT NULL DEFAULT '',\n    notice_chat_id INTEGER,\n    notice_message_id INTEGER,\n    dm_message_id INTEGER\n)"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE tokens(\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    chat_id INTEGER NOT NULL,\n    token TEXT NOT NULL,\n    name TEXT,\n    issuer_user_id INTEGER,\n    issuer_name TEXT,\n    issued_at INTEGER NOT NULL,\n    last_used_at INTEGER,\n    revoked_at INTEGER\n)"
    )
    .execute(&pool)
    .await
    .unwrap();

    Database::new(pool)
}
