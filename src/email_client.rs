use crate::domain::SubscriberEmail;
use reqwest::{Client, Error};
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();

        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), Error> {
        let url = format!("{}/email", &self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(&url)
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
    use claims::{assert_err, assert_ok};
    use helpers::{content, email, email_client, subject, SendEmailBodyMatcher};
    use std::time::Duration;
    use wiremock::{
        matchers::{any, header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // given
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // when
        let response = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // then
        assert_ok!(response);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // given
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // when
        let response = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // then
        assert_err!(response);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // given
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(300)))
            .expect(1)
            .mount(&mock_server)
            .await;

        // when
        let response = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // then
        assert_err!(response);
    }

    mod helpers {
        use crate::{domain::SubscriberEmail, email_client::EmailClient};
        use fake::{
            faker::{
                internet::en::SafeEmail,
                lorem::en::{Paragraph, Sentence},
            },
            Fake, Faker,
        };
        use secrecy::Secret;
        use serde_json::{from_slice, Value};
        use std::time::Duration;
        use wiremock::{Match, Request};

        pub struct SendEmailBodyMatcher;

        impl Match for SendEmailBodyMatcher {
            fn matches(&self, request: &Request) -> bool {
                let result: Result<Value, _> = from_slice(&request.body);

                if let Ok(body) = result {
                    ["From", "To", "Subject", "HtmlBody", "TextBody"]
                        .iter()
                        .all(|&value| body.get(value).is_some())
                } else {
                    false
                }
            }
        }

        pub fn email_client(base_url: String) -> EmailClient {
            EmailClient::new(
                base_url,
                email(),
                Secret::new(Faker.fake()),
                Duration::from_millis(200),
            )
        }

        pub fn email() -> SubscriberEmail {
            SubscriberEmail::parse(SafeEmail().fake()).unwrap()
        }

        pub fn subject() -> String {
            Sentence(1..2).fake()
        }

        pub fn content() -> String {
            Paragraph(1..10).fake()
        }
    }
}
