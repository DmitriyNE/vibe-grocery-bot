CREATE TABLE IF NOT EXISTS tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    token TEXT NOT NULL,
    issued_at INTEGER NOT NULL,
    last_used_at INTEGER,
    revoked_at INTEGER
);

CREATE INDEX IF NOT EXISTS tokens_chat_id_index ON tokens(chat_id);
