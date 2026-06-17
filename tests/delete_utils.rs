use reqwest::Client;
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

    let client = Client::builder().no_proxy().build().unwrap();
    let bot =
        Bot::with_client("TEST", client).set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
    let handle = delete_after(bot, ChatId(1), MessageId(2), 0);
    handle.await.unwrap();
    server.verify().await;
}
