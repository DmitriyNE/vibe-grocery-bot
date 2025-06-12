use super::Database;
use crate::db::types::{ChatKey, ItemId};
use anyhow::Result;
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
    pub chat_id: ChatKey,
    pub selected: HashSet<ItemId>,
    pub notice: Option<(ChatId, MessageId)>,
    pub dm_message_id: Option<MessageId>,
}

fn parse_selected(s: &str) -> HashSet<ItemId> {
    s.split(',')
        .filter_map(|p| p.parse::<i64>().ok().map(ItemId))
        .collect()
}

fn join_selected(set: &HashSet<ItemId>) -> String {
    let mut ids: Vec<i64> = set.iter().copied().map(Into::into).collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

impl Database {
    pub async fn init_delete_session(&self, user_id: i64, chat_id: ChatKey) -> Result<()> {
        tracing::debug!(user_id, chat_id = chat_id.0, "Initializing delete session");
        sqlx::query(
            "INSERT INTO delete_session (user_id, chat_id, selected) VALUES (?, ?, '') \
             ON CONFLICT(user_id) DO UPDATE SET chat_id=excluded.chat_id, selected='', notice_chat_id=NULL, notice_message_id=NULL, dm_message_id=NULL",
        )
        .bind(user_id)
        .bind::<i64>(chat_id.into())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_delete_selection(
        &self,
        user_id: i64,
        selected: &HashSet<ItemId>,
    ) -> Result<()> {
        let joined = join_selected(selected);
        tracing::trace!(user_id, selection=?joined, "Updating delete selection");
        sqlx::query("UPDATE delete_session SET selected = ? WHERE user_id = ?")
            .bind(joined)
            .bind(user_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn set_delete_notice(
        &self,
        user_id: i64,
        chat_id: ChatKey,
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
        .bind::<i64>(chat_id.into())
        .bind(message_id.0)
        .bind(user_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn set_delete_dm_message(&self, user_id: i64, message_id: MessageId) -> Result<()> {
        tracing::debug!(
            user_id,
            message_id = message_id.0,
            "Setting delete DM message"
        );
        sqlx::query("UPDATE delete_session SET dm_message_id = ? WHERE user_id = ?")
            .bind(message_id.0)
            .bind(user_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn get_delete_session(&self, user_id: i64) -> Result<Option<DeleteSession>> {
        tracing::trace!(user_id, "Fetching delete session");
        if let Some(row) = sqlx::query_as::<_, DeleteSessionRow>(
            "SELECT chat_id, selected, notice_chat_id, notice_message_id, dm_message_id FROM delete_session WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?
        {
            let notice = match (row.notice_chat_id, row.notice_message_id) {
                (Some(c), Some(m)) => Some((ChatId(c), MessageId(m))),
                _ => None,
            };
            Ok(Some(DeleteSession {
                chat_id: ChatKey(row.chat_id),
                selected: parse_selected(&row.selected),
                notice,
                dm_message_id: row.dm_message_id.map(MessageId),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn clear_delete_session(&self, user_id: i64) -> Result<()> {
        tracing::debug!(user_id, "Clearing delete session");
        sqlx::query("DELETE FROM delete_session WHERE user_id = ?")
            .bind(user_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{ChatKey, ItemId};
    use crate::tests::util::init_test_db;
    use proptest::prelude::*;
    use teloxide::types::{ChatId, MessageId};

    #[test]
    fn parse_selected_empty() {
        let set = parse_selected("");
        assert!(set.is_empty());
    }

    #[test]
    fn join_selected_sorts() {
        let mut set = HashSet::new();
        set.insert(ItemId(5));
        set.insert(ItemId(3));
        set.insert(ItemId(7));
        assert_eq!(join_selected(&set), "3,5,7");
    }

    #[test]
    fn parse_join_roundtrip() {
        let mut original = HashSet::new();
        original.insert(ItemId(2));
        original.insert(ItemId(1));
        original.insert(ItemId(9));
        let joined = join_selected(&original);
        let parsed = parse_selected(&joined);
        assert_eq!(original, parsed);
    }

    proptest! {
        #[test]
        fn prop_parse_join_roundtrip(set in proptest::collection::hash_set(0i64..1000, 0..20)) {
            let set: HashSet<ItemId> = set.into_iter().map(ItemId).collect();
            let joined = join_selected(&set);
            let parsed = parse_selected(&joined);
            prop_assert_eq!(set, parsed);
        }

        #[test]
        fn prop_join_selected_sorted(set in proptest::collection::hash_set(-1000i64..1000, 0..20)) {
            let set: HashSet<ItemId> = set.into_iter().map(ItemId).collect();
            let joined = join_selected(&set);
            let parsed: Vec<i64> = if joined.is_empty() {
                Vec::new()
            } else {
                joined.split(',').map(|s| s.parse().unwrap()).collect()
            };
            let mut expected: Vec<i64> = set.iter().map(|i| i.0).collect();
            expected.sort_unstable();
            prop_assert_eq!(parsed, expected);
        }
    }

    #[tokio::test]
    async fn delete_session_roundtrip() -> Result<()> {
        let db = init_test_db().await;
        let user = 1i64;
        let chat_a = ChatId(10);
        db.init_delete_session(user, ChatKey(chat_a.0)).await?;

        let mut session = db.get_delete_session(user).await?.unwrap();
        assert_eq!(ChatId::from(session.chat_id), chat_a);
        assert!(session.selected.is_empty());
        assert!(session.notice.is_none());
        assert!(session.dm_message_id.is_none());

        let mut selected = HashSet::new();
        selected.insert(ItemId(5));
        selected.insert(ItemId(7));
        db.update_delete_selection(user, &selected).await?;

        session = db.get_delete_session(user).await?.unwrap();
        assert_eq!(session.selected, selected);

        db.set_delete_notice(user, ChatKey(20), MessageId(3))
            .await?;
        db.set_delete_dm_message(user, MessageId(4)).await?;

        session = db.get_delete_session(user).await?.unwrap();
        assert_eq!(session.notice, Some((ChatId(20), MessageId(3))));
        assert_eq!(session.dm_message_id, Some(MessageId(4)));

        db.clear_delete_session(user).await?;
        assert!(db.get_delete_session(user).await?.is_none());

        Ok(())
    }
}
