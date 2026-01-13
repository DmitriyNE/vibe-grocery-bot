use axum::body::Body;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use serde_json::json;
use shopbot::tests::util::init_test_db;
use shopbot::{api_router, ApiConfig};
use teloxide::types::ChatId;
use tower::ServiceExt;

#[tokio::test]
async fn api_add_toggle_delete_flow() {
    let db = init_test_db().await;
    let chat_id = ChatId(70);
    db.create_token(chat_id, "token-flow", 1).await.unwrap();

    let app = api_router(
        db.clone(),
        ApiConfig {
            rate_limit_per_second: None,
        },
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/add")
                .header(AUTHORIZATION, "Bearer token-flow")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "text": "Granola" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let items = db.list_items(chat_id).await.unwrap();
    assert_eq!(items.len(), 1);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/toggle")
                .header(AUTHORIZATION, "Bearer token-flow")
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
    assert!(items[0].done);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/delete")
                .header(AUTHORIZATION, "Bearer token-flow")
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
async fn api_rate_limit_rejects_second_request() {
    let db = init_test_db().await;
    let chat_id = ChatId(71);
    db.create_token(chat_id, "token-rate", 1).await.unwrap();

    let app = api_router(
        db,
        ApiConfig {
            rate_limit_per_second: Some(1),
        },
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/list")
                .header(AUTHORIZATION, "Bearer token-rate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/list")
                .header(AUTHORIZATION, "Bearer token-rate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
