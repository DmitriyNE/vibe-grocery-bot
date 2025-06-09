use shopbot::delete_after;
use teloxide::{prelude::*, types::MessageId};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_delete_after_sends_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/botTEST/DeleteMessage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    delete_after(bot, ChatId(1), MessageId(2), 0);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    server.verify().await;
}
