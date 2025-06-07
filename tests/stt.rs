use shopbot::stt::transcribe_audio_test;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_transcribe_audio_parsing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"text":"milk"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/v1/audio/transcriptions", server.uri());
    let res = transcribe_audio_test("whisper-1", "k", None, b"123", &url)
        .await
        .unwrap();
    assert_eq!(res, "milk");
}
