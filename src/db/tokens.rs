use super::Database;
use anyhow::Result;
use teloxide::types::ChatId;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct TokenRecord {
    pub id: i64,
    pub chat_id: i64,
    pub token: String,
    pub issued_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

impl Database {
    pub async fn create_token(&self, chat_id: ChatId, token: &str, issued_at: i64) -> Result<()> {
        tracing::debug!(chat_id = chat_id.0, issued_at, "Creating token for chat");
        sqlx::query("INSERT INTO tokens (chat_id, token, issued_at) VALUES (?, ?, ?)")
            .bind(chat_id.0)
            .bind(token)
            .bind(issued_at)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_tokens(&self, chat_id: ChatId) -> Result<Vec<TokenRecord>> {
        tracing::trace!(chat_id = chat_id.0, "Listing tokens");
        sqlx::query_as(
            "SELECT id, chat_id, token, issued_at, last_used_at, revoked_at \
             FROM tokens WHERE chat_id = ? ORDER BY issued_at DESC, id DESC",
        )
        .bind(chat_id.0)
        .fetch_all(self.pool())
        .await
        .map_err(Into::into)
    }

    pub async fn revoke_token(
        &self,
        chat_id: ChatId,
        token: &str,
        revoked_at: i64,
    ) -> Result<bool> {
        tracing::debug!(chat_id = chat_id.0, revoked_at, "Revoking token for chat");
        let result = sqlx::query(
            "UPDATE tokens SET revoked_at = ? WHERE chat_id = ? AND token = ? AND revoked_at IS NULL",
        )
        .bind(revoked_at)
        .bind(chat_id.0)
        .bind(token)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn use_token(&self, token: &str, used_at: i64) -> Result<Option<ChatId>> {
        let chat_id: Option<i64> =
            sqlx::query_scalar("SELECT chat_id FROM tokens WHERE token = ? AND revoked_at IS NULL")
                .bind(token)
                .fetch_optional(self.pool())
                .await?;

        if let Some(chat_id) = chat_id {
            sqlx::query(
                "UPDATE tokens SET last_used_at = ? WHERE token = ? AND revoked_at IS NULL",
            )
            .bind(used_at)
            .bind(token)
            .execute(self.pool())
            .await?;
            tracing::debug!(chat_id, used_at, "Updated token last_used_at");
            return Ok(Some(ChatId(chat_id)));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;
    use proptest::prelude::*;
    use teloxide::types::ChatId;

    #[tokio::test]
    async fn token_create_and_list() -> Result<()> {
        let db = init_test_db().await;
        let chat_id = ChatId(42);
        db.create_token(chat_id, "token-a", 100).await?;
        db.create_token(chat_id, "token-b", 200).await?;

        let tokens = db.list_tokens(chat_id).await?;
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, "token-b");
        assert_eq!(tokens[1].token, "token-a");
        Ok(())
    }

    #[tokio::test]
    async fn token_revoke() -> Result<()> {
        let db = init_test_db().await;
        let chat_id = ChatId(7);
        db.create_token(chat_id, "token-x", 123).await?;

        let revoked = db.revoke_token(chat_id, "token-x", 456).await?;
        assert!(revoked);

        let tokens = db.list_tokens(chat_id).await?;
        assert_eq!(tokens[0].revoked_at, Some(456));
        Ok(())
    }

    #[tokio::test]
    async fn token_use_updates_last_used() -> Result<()> {
        let db = init_test_db().await;
        let chat_id = ChatId(9);
        db.create_token(chat_id, "token-use", 123).await?;

        let used_at = 555;
        let resolved = db.use_token("token-use", used_at).await?;
        assert_eq!(resolved, Some(chat_id));

        let tokens = db.list_tokens(chat_id).await?;
        assert_eq!(tokens[0].last_used_at, Some(used_at));
        Ok(())
    }

    proptest! {
        #[test]
        fn prop_list_tokens_ordered(issued_at_values in proptest::collection::vec(-10000i64..10000, 0..20)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = init_test_db().await;
                let chat_id = ChatId(1);
                for (idx, issued_at) in issued_at_values.iter().enumerate() {
                    let token = format!("token-{idx}");
                    db.create_token(chat_id, &token, *issued_at).await.unwrap();
                }

                let mut expected: Vec<(i64, usize)> = issued_at_values
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(idx, ts)| (ts, idx))
                    .collect();
                expected.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

                let tokens = db.list_tokens(chat_id).await.unwrap();
                let listed: Vec<String> = tokens.into_iter().map(|token| token.token).collect();
                let expected_tokens: Vec<String> = expected
                    .iter()
                    .map(|(_, idx)| format!("token-{idx}"))
                    .collect();
                prop_assert_eq!(listed, expected_tokens);
                Ok(())
            })?;
        }
    }
}
