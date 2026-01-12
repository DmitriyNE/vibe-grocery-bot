use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::{Database, Item};

#[derive(Debug, Serialize, Deserialize)]
struct ApiItem {
    id: i64,
    text: String,
    done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse {
    items: Vec<ApiItem>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
}

pub fn router(db: Database) -> Router {
    Router::new()
        .route("/api/list", get(get_list))
        .with_state(db)
}

async fn get_list(State(db): State<Database>, headers: HeaderMap) -> Response {
    let token = match extract_bearer_token(&headers) {
        Some(token) => token,
        None => {
            tracing::debug!("Missing bearer token");
            return unauthorized_response();
        }
    };

    tracing::debug!(token_preview = %token_preview(&token), "Checking bearer token");
    let used_at = chrono::Utc::now().timestamp();
    let chat_id = match db.use_token(&token, used_at).await {
        Ok(Some(chat_id)) => chat_id,
        Ok(None) => {
            tracing::debug!("Bearer token rejected");
            return unauthorized_response();
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to validate bearer token");
            return internal_error_response();
        }
    };

    let items = match db.list_items(chat_id).await {
        Ok(items) => items,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to load items");
            return internal_error_response();
        }
    };

    tracing::debug!(
        chat_id = chat_id.0,
        item_count = items.len(),
        "Read list items"
    );
    let response = ListResponse {
        items: items.into_iter().map(ApiItem::from).collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        })
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

fn token_preview(token: &str) -> String {
    token.chars().take(6).collect()
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "unauthorized",
        }),
    )
        .into_response()
}

fn internal_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal_error",
        }),
    )
        .into_response()
}

impl From<Item> for ApiItem {
    fn from(item: Item) -> Self {
        Self {
            id: item.id,
            text: item.text,
            done: item.done,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use proptest::prelude::*;
    use teloxide::types::ChatId;
    use tower::ServiceExt;

    #[tokio::test]
    async fn list_requires_auth() {
        let db = init_test_db().await;
        let app = router(db);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_returns_items() {
        let db = init_test_db().await;
        let chat_id = ChatId(10);
        db.create_token(chat_id, "token-123", 1).await.unwrap();
        db.add_item(chat_id, "Milk").await.unwrap();

        let app = router(db.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/list")
                    .header(AUTHORIZATION, "Bearer token-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: ListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.items.len(), 1);
        assert_eq!(payload.items[0].text, "Milk");

        let tokens = db.list_tokens(chat_id).await.unwrap();
        assert!(tokens[0].last_used_at.is_some());
    }

    #[tokio::test]
    async fn list_rejects_invalid_token() {
        let db = init_test_db().await;
        let chat_id = ChatId(7);
        db.create_token(chat_id, "token-abc", 1).await.unwrap();

        let app = router(db.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/list")
                    .header(AUTHORIZATION, "Bearer wrong-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let tokens = db.list_tokens(chat_id).await.unwrap();
        assert!(tokens[0].last_used_at.is_none());
    }

    #[tokio::test]
    async fn list_allows_empty_response() {
        let db = init_test_db().await;
        let chat_id = ChatId(42);
        db.create_token(chat_id, "token-empty", 1).await.unwrap();

        let app = router(db);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/list")
                    .header(AUTHORIZATION, "Bearer token-empty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: ListResponse = serde_json::from_slice(&body).unwrap();
        assert!(payload.items.is_empty());
    }

    proptest! {
        #[test]
        fn bearer_token_parses_from_header(token in "[A-Za-z0-9_-]{1,64}") {
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
            let parsed = extract_bearer_token(&headers);
            prop_assert_eq!(parsed, Some(token));
        }
    }
}
