use axum::{
    body::Body,
    extract::{Extension, State},
    http::{header::AUTHORIZATION, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use teloxide::types::ChatId;

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

#[derive(Debug, Deserialize)]
struct AddRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ToggleRequest {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct DeleteRequest {
    id: i64,
}

#[derive(Debug, Serialize)]
struct MutationResponse {
    affected: u64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct AuthenticatedChatId(ChatId);

pub fn router(db: Database) -> Router {
    let auth_layer = middleware::from_fn_with_state(db.clone(), require_auth);
    Router::new()
        .route("/api/list", get(get_list))
        .route("/api/add", post(add_item))
        .route("/api/toggle", post(toggle_item))
        .route("/api/delete", post(delete_item))
        .route("/api/archive", post(archive_list))
        .route("/api/nuke", post(nuke_list))
        .route("/api/done", post(done_list))
        .layer(auth_layer)
        .with_state(db)
}

async fn require_auth(State(db): State<Database>, mut req: Request<Body>, next: Next) -> Response {
    let token = match extract_bearer_token(req.headers()) {
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

    tracing::debug!(chat_id = chat_id.0, "Authenticated API request");
    req.extensions_mut().insert(AuthenticatedChatId(chat_id));
    next.run(req).await
}

async fn get_list(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
) -> Response {
    let chat_id = auth.0;
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

async fn add_item(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
    Json(payload): Json<AddRequest>,
) -> Response {
    let chat_id = auth.0;
    let text = payload.text.trim();
    if text.is_empty() {
        return bad_request_response();
    }

    let affected = match db.add_item_count(chat_id, text).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to add item");
            return internal_error_response();
        }
    };

    tracing::debug!(
        chat_id = chat_id.0,
        affected,
        text = %text,
        "Added item via API"
    );
    (StatusCode::CREATED, Json(MutationResponse { affected })).into_response()
}

async fn toggle_item(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
    Json(payload): Json<ToggleRequest>,
) -> Response {
    let chat_id = auth.0;
    let affected = match db.toggle_item_count(chat_id, payload.id).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to toggle item");
            return internal_error_response();
        }
    };

    tracing::debug!(
        chat_id = chat_id.0,
        item_id = payload.id,
        affected,
        "Toggled item via API"
    );
    if affected == 0 {
        return not_found_response();
    }
    (StatusCode::OK, Json(MutationResponse { affected })).into_response()
}

async fn delete_item(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
    Json(payload): Json<DeleteRequest>,
) -> Response {
    let chat_id = auth.0;
    let affected = match db.delete_item_count(chat_id, payload.id).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to delete item");
            return internal_error_response();
        }
    };

    tracing::debug!(
        chat_id = chat_id.0,
        item_id = payload.id,
        affected,
        "Deleted item via API"
    );
    if affected == 0 {
        return not_found_response();
    }
    (StatusCode::OK, Json(MutationResponse { affected })).into_response()
}

async fn archive_list(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
) -> Response {
    let chat_id = auth.0;
    let affected = match db.delete_all_items_count(chat_id).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to archive list");
            return internal_error_response();
        }
    };
    tracing::debug!(chat_id = chat_id.0, affected, "Archived list via API");
    (StatusCode::OK, Json(MutationResponse { affected })).into_response()
}

async fn nuke_list(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
) -> Response {
    let chat_id = auth.0;
    let affected = match db.delete_all_items_count(chat_id).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to nuke list");
            return internal_error_response();
        }
    };
    tracing::debug!(chat_id = chat_id.0, affected, "Nuked list via API");
    (StatusCode::OK, Json(MutationResponse { affected })).into_response()
}

async fn done_list(
    State(db): State<Database>,
    Extension(auth): Extension<AuthenticatedChatId>,
) -> Response {
    let chat_id = auth.0;
    let items = match db.list_items(chat_id).await {
        Ok(items) => items,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to load items");
            return internal_error_response();
        }
    };

    let done_ids: Vec<i64> = items
        .iter()
        .filter(|item| item.done)
        .map(|i| i.id)
        .collect();
    let affected = match db.delete_items_count(chat_id, &done_ids).await {
        Ok(affected) => affected,
        Err(err) => {
            tracing::error!(error = %err, chat_id = chat_id.0, "Failed to archive checked items");
            return internal_error_response();
        }
    };

    tracing::debug!(
        chat_id = chat_id.0,
        affected,
        done_count = done_ids.len(),
        "Archived checked items via API"
    );
    (StatusCode::OK, Json(MutationResponse { affected })).into_response()
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

fn bad_request_response() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_request",
        }),
    )
        .into_response()
}

fn not_found_response() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: "not_found" }),
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
    use serde_json::json;
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
    async fn mutations_require_auth() {
        let db = init_test_db().await;
        let app = router(db);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/add")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"Milk"}"#))
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
    async fn add_toggle_delete_flow() {
        let db = init_test_db().await;
        let chat_id = ChatId(11);
        db.create_token(chat_id, "token-add", 1).await.unwrap();
        let app = router(db.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/add")
                    .header(AUTHORIZATION, "Bearer token-add")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "text": "Oats" })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let items = db.list_items(chat_id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].done);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/toggle")
                    .header(AUTHORIZATION, "Bearer token-add")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "id": items[0].id })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let items = db.list_items(chat_id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].done);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/delete")
                    .header(AUTHORIZATION, "Bearer token-add")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "id": items[0].id })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let items = db.list_items(chat_id).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn done_archives_checked_items() {
        let db = init_test_db().await;
        let chat_id = ChatId(12);
        db.create_token(chat_id, "token-done", 1).await.unwrap();
        db.add_item(chat_id, "Tea").await.unwrap();
        db.add_item(chat_id, "Sugar").await.unwrap();
        let items = db.list_items(chat_id).await.unwrap();
        db.toggle_item(chat_id, items[0].id).await.unwrap();

        let app = router(db.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/done")
                    .header(AUTHORIZATION, "Bearer token-done")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let items = db.list_items(chat_id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].done);
        assert_eq!(items[0].text, "Sugar");
    }

    #[tokio::test]
    async fn archive_and_nuke_clear_items() {
        let db = init_test_db().await;
        let chat_id = ChatId(13);
        db.create_token(chat_id, "token-archive", 1).await.unwrap();
        db.add_item(chat_id, "Bread").await.unwrap();
        let app = router(db.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/archive")
                    .header(AUTHORIZATION, "Bearer token-archive")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(db.list_items(chat_id).await.unwrap().is_empty());

        db.add_item(chat_id, "Butter").await.unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nuke")
                    .header(AUTHORIZATION, "Bearer token-archive")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(db.list_items(chat_id).await.unwrap().is_empty());
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
