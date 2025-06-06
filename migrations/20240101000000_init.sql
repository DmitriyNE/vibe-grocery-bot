-- Initial schema
CREATE TABLE IF NOT EXISTS items (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id   INTEGER NOT NULL,
    text      TEXT    NOT NULL,
    done      BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS chat_state (
    chat_id              INTEGER PRIMARY KEY,
    last_list_message_id INTEGER
);

CREATE TABLE IF NOT EXISTS delete_session (
    user_id INTEGER PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    selected TEXT NOT NULL DEFAULT '',
    notice_chat_id INTEGER,
    notice_message_id INTEGER
);
