use shopbot::ai::gpt::parse_items_gpt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_parse_items_gpt_numbers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"choices":[{"message":{"content":"{\"items\":[\"one milk\",\"2 eggs\"]}"}}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let url = format!("{}/v1/chat/completions", server.uri());
    let items = parse_items_gpt("k", "gpt-4.1", "one milk and 2 eggs", Some(&url))
        .await
        .unwrap();
    assert_eq!(items, vec!["one milk", "2 eggs"]);
}
