use crate::helpers::TestApp;
use linkify::{LinkFinder, LinkKind};
use serde_json::Value;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // when
    let response = app.post_subscriptions(body.into()).await;

    // then
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // when
    app.post_subscriptions(body.into()).await;

    // then
    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.name, "Imię Nazwisko");
    assert_eq!(saved.email, "imie.nazwisko@example.com");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("name=Imi%C4%99%20Nazwisko&email=", "empty email"),
        (
            "name=Imi%C4%99%20Nazwisko&email=definitely-not-an-email",
            "invalid email",
        ),
        ("name=&email=imie.nazwisko%40example.com", "empty name"),
        ("name=&email=", "empty both name and email"),
    ];

    for (body, description) in test_cases {
        // when
        let response = app.post_subscriptions(body.into()).await;

        // then
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BAD_REQUEST when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("name=Imi%C4%99%20Nazwisko", "missing the email"),
        ("email=imie.nazwisko%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, message) in test_cases {
        // when
        let response = app.post_subscriptions(body.into()).await;

        // then
        assert_eq!(
            response.status(),
            422,
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            message
        );
        let saved = sqlx::query!("SELECT name, email FROM subscriptions")
            .fetch_optional(&app.db_pool)
            .await
            .expect("Failed to fetch unsaved subscription");
        assert!(saved.is_none());
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // when
    app.post_subscriptions(body.into()).await;

    // then assert
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // when
    app.post_subscriptions(body.into()).await;

    // then
    let request = &app.email_server.received_requests().await.unwrap()[0];
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    let get_link = |s: &str| {
        let links: Vec<_> = LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(&body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
