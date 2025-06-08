use shopbot::ai::gpt::{interpret_voice_command_test, VoiceCommand};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_voice_command_delete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"choices":[{"message":{"content":"{\"delete\":[\"Milk\"]}"}}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let url = format!("{}/v1/chat/completions", server.uri());
    let res = interpret_voice_command_test(
        "k",
        "gpt-4.1",
        "delete milk",
        &["Milk".to_string(), "Bread".to_string()],
        &url,
    )
    .await
    .unwrap();
    assert_eq!(res, VoiceCommand::Delete(vec!["Milk".to_string()]));
}

#[tokio::test]
async fn test_voice_command_add() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"choices":[{"message":{"content":"{\"add\":[\"Apples\"]}"}}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let url = format!("{}/v1/chat/completions", server.uri());
    let res = interpret_voice_command_test(
        "k",
        "gpt-4.1",
        "apples",
        &["Milk".to_string(), "Bread".to_string()],
        &url,
    )
    .await
    .unwrap();
    assert_eq!(res, VoiceCommand::Add(vec!["Apples".to_string()]));
}
