use shopbot::ai::vision::parse_photo_items_test;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_parse_photo_items() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"choices":[{"message":{"content":"{\"items\":[\"apples\"]}"}}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let url = format!("{}/v1/chat/completions", server.uri());
    let items = parse_photo_items_test("k", b"img", &url).await.unwrap();
    assert_eq!(items, vec!["apples"]);
}
