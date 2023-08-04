use secrecy::{ExposeSecret, Secret};
use serde::Serialize;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: reqwest::Client,
    // ? This depends on an external crate. Do we need abstraction?
    sender: SubscriberEmail,
    // ? SubscriberEmail is a domain type, so why is it used here?
    base_url: String,
    // external service url to send email
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        time_out: std::time::Duration,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(time_out)
            .build()
            .unwrap();
        Self {
            http_client,
            sender,
            base_url,
            authorization_token,
        }
    }

    fn url(&self) -> String {
        format!("{}/email", self.base_url)
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let _request_result = self
            .http_client
            .post(&self.url())
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

/// # Postmark API
///
/// ```shell
/// curl "https://api.postmarkapp.com/email" \
///   -X POST \
///   -H "Accept: application/json" \
///   -H "Content-Type: application/json" \
///   -H "X-Postmark-Server-Token: xxx-yyy-zzz" \
///   -d '{
///         "From": "info@example.com",
///         "To": "dummy_destination@gmail.com",
///         "Subject": "Hello from Postmark",
///         "HtmlBody": "<strong>Hello</strong> dear Postmark user.",
///         "MessageStream": "broadcast"
///       }'
/// ```
///
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::http::Method;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                let json_keys = ["From", "To", "Subject", "HtmlBody", "TextBody"];
                json_keys.into_iter().all(|k| body.get(k).is_some())
            } else {
                false
            }
        }
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn authorization_token() -> Secret<String> {
        Secret::new(Faker.fake())
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn timeout_ms(timeout_milliseconds: u64) -> Duration {
        Duration::from_millis(timeout_milliseconds)
    }

    fn email_client(base_url: String, timeout_milliseconds: u64) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            authorization_token(),
            timeout_ms(timeout_milliseconds),
        )
    }

    async fn send_email_for_testing(email_client: EmailClient) -> Result<(), reqwest::Error> {
        email_client
            .send_email(email(), &subject(), &content(), &content())
            .await
    }

    #[tokio::test]
    async fn send_email_fires_request() {
        // Mockserver
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().to_string(), 200);

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method(Method::Post))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        assert_ok!(send_email_for_testing(email_client).await)
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().to_string(), 200);

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        assert_ok!(send_email_for_testing(email_client).await)
    }

    #[tokio::test]
    async fn send_email_gets_error_when_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().to_string(), 200);

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(send_email_for_testing(email_client).await);
    }
}
