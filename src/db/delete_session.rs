use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use teloxide::types::{ChatId, MessageId};

#[derive(sqlx::FromRow)]
struct DeleteSessionRow {
    chat_id: i64,
    selected: String,
    notice_chat_id: Option<i64>,
    notice_message_id: Option<i32>,
    dm_message_id: Option<i32>,
}

pub struct DeleteSession {
    pub chat_id: ChatId,
    pub selected: HashSet<i64>,
    pub notice: Option<(ChatId, MessageId)>,
    pub dm_message_id: Option<MessageId>,
}

fn parse_selected(s: &str) -> HashSet<i64> {
    s.split(',').filter_map(|p| p.parse::<i64>().ok()).collect()
}

fn join_selected(set: &HashSet<i64>) -> String {
    let mut ids: Vec<i64> = set.iter().copied().collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub async fn init_delete_session(db: &Pool<Sqlite>, user_id: i64, chat_id: ChatId) -> Result<()> {
    tracing::debug!(user_id, chat_id = chat_id.0, "Initializing delete session");
    sqlx::query(
        "INSERT INTO delete_session (user_id, chat_id, selected) VALUES (?, ?, '') \
         ON CONFLICT(user_id) DO UPDATE SET chat_id=excluded.chat_id, selected='', notice_chat_id=NULL, notice_message_id=NULL, dm_message_id=NULL",
    )
    .bind(user_id)
    .bind(chat_id.0)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_delete_selection(
    db: &Pool<Sqlite>,
    user_id: i64,
    selected: &HashSet<i64>,
) -> Result<()> {
    tracing::trace!(user_id, selection=?selected, "Updating delete selection");
    let joined = join_selected(selected);
    sqlx::query("UPDATE delete_session SET selected = ? WHERE user_id = ?")
        .bind(joined)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_delete_notice(
    db: &Pool<Sqlite>,
    user_id: i64,
    chat_id: ChatId,
    message_id: MessageId,
) -> Result<()> {
    tracing::debug!(
        user_id,
        chat_id = chat_id.0,
        message_id = message_id.0,
        "Setting delete notice",
    );
    sqlx::query(
        "UPDATE delete_session SET notice_chat_id = ?, notice_message_id = ? WHERE user_id = ?",
    )
    .bind(chat_id.0)
    .bind(message_id.0)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_delete_dm_message(
    db: &Pool<Sqlite>,
    user_id: i64,
    message_id: MessageId,
) -> Result<()> {
    tracing::debug!(
        user_id,
        message_id = message_id.0,
        "Setting delete DM message"
    );
    sqlx::query("UPDATE delete_session SET dm_message_id = ? WHERE user_id = ?")
        .bind(message_id.0)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_delete_session(db: &Pool<Sqlite>, user_id: i64) -> Result<Option<DeleteSession>> {
    tracing::trace!(user_id, "Fetching delete session");
    if let Some(row) = sqlx::query_as::<_, DeleteSessionRow>(
        "SELECT chat_id, selected, notice_chat_id, notice_message_id, dm_message_id FROM delete_session WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    {
        let notice = match (row.notice_chat_id, row.notice_message_id) {
            (Some(c), Some(m)) => Some((ChatId(c), MessageId(m))),
            _ => None,
        };
        Ok(Some(DeleteSession {
            chat_id: ChatId(row.chat_id),
            selected: parse_selected(&row.selected),
            notice,
            dm_message_id: row.dm_message_id.map(MessageId),
        }))
    } else {
        Ok(None)
    }
}

pub async fn clear_delete_session(db: &Pool<Sqlite>, user_id: i64) -> Result<()> {
    tracing::debug!(user_id, "Clearing delete session");
    sqlx::query("DELETE FROM delete_session WHERE user_id = ?")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
    use teloxide::types::{ChatId, MessageId};

    #[test]
    fn parse_selected_empty() {
        let set = parse_selected("");
        assert!(set.is_empty());
    }

    #[test]
    fn join_selected_sorts() {
        let mut set = HashSet::new();
        set.insert(5);
        set.insert(3);
        set.insert(7);
        assert_eq!(join_selected(&set), "3,5,7");
    }

    #[test]
    fn parse_join_roundtrip() {
        let mut original = HashSet::new();
        original.insert(2);
        original.insert(1);
        original.insert(9);
        let joined = join_selected(&original);
        let parsed = parse_selected(&joined);
        assert_eq!(original, parsed);
    }

    proptest! {
        #[test]
        fn prop_parse_join_roundtrip(set in proptest::collection::hash_set(0i64..1000, 0..20)) {
            let joined = join_selected(&set);
            let parsed = parse_selected(&joined);
            prop_assert_eq!(set, parsed);
        }

        #[test]
        fn prop_join_selected_sorted(set in proptest::collection::hash_set(-1000i64..1000, 0..20)) {
            let joined = join_selected(&set);
            let parsed: Vec<i64> = if joined.is_empty() {
                Vec::new()
            } else {
                joined.split(',').map(|s| s.parse().unwrap()).collect()
            };
            let mut expected: Vec<i64> = set.iter().copied().collect();
            expected.sort_unstable();
            prop_assert_eq!(parsed, expected);
        }
    }

    async fn setup_db() -> Pool<Sqlite> {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE delete_session(
                user_id INTEGER PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                selected TEXT NOT NULL DEFAULT '',
                notice_chat_id INTEGER,
                notice_message_id INTEGER,
                dm_message_id INTEGER
            )",
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn delete_session_roundtrip() -> Result<()> {
        let db = setup_db().await;
        let user = 1i64;
        let chat_a = ChatId(10);
        init_delete_session(&db, user, chat_a).await?;

        let mut session = get_delete_session(&db, user).await?.unwrap();
        assert_eq!(session.chat_id, chat_a);
        assert!(session.selected.is_empty());
        assert!(session.notice.is_none());
        assert!(session.dm_message_id.is_none());

        let mut selected = HashSet::new();
        selected.insert(5);
        selected.insert(7);
        update_delete_selection(&db, user, &selected).await?;

        session = get_delete_session(&db, user).await?.unwrap();
        assert_eq!(session.selected, selected);

        set_delete_notice(&db, user, ChatId(20), MessageId(3)).await?;
        set_delete_dm_message(&db, user, MessageId(4)).await?;

        session = get_delete_session(&db, user).await?.unwrap();
        assert_eq!(session.notice, Some((ChatId(20), MessageId(3))));
        assert_eq!(session.dm_message_id, Some(MessageId(4)));

        clear_delete_session(&db, user).await?;
        assert!(get_delete_session(&db, user).await?.is_none());

        Ok(())
    }
}
