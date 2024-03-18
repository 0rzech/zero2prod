use crate::helpers::TestApp;
use claims::assert_ge;
use regex::Regex;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};
use zero2prod::domain::{token_regex, SubscriptionStatus};

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
            response.status(),
            400,
            "The API did not return a 400 BAD_REQUEST when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_validation_error_description_when_fields_are_present_but_empty() {
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
        let content_length = response.content_length().unwrap();

        // then
        assert_ge!(
            content_length,
            0,
            "The API did not return validation error description when the payload was {}",
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
async fn subscribe_sends_a_confirmation_email_on_repeated_calls() {
    // given
    let app = TestApp::spawn().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(3)
        .mount(&app.email_server)
        .await;

    for i in 1..4 {
        // when
        let body = format!("name=Imi%C4%99%20Nazwisko%20{i}&email=imie.nazwisko.{i}%40example.com");
        let response = app.post_subscriptions(body).await;

        // then
        assert_eq!(
            response.status(),
            200,
            "The API did not reurn a 200 OK for call number {}",
            i
        );
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

    // then assert mock
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
    let links = app.get_confirmation_links_from_email_request().await;
    let re = Regex::new(format!(r"\?subscription_token={}$", token_regex()).as_str()).unwrap();

    assert_eq!(links.html, links.plain_text);
    assert!(
        re.is_match(links.html.as_str()),
        "`{}` didn't match `{}` regex",
        links.html,
        re
    );
}

#[tokio::test]
async fn subscribe_does_not_send_confirmation_email_when_email_already_confirmed() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = $1
        WHERE email = 'imie.nazwisko@example.com'
        "#,
        SubscriptionStatus::Confirmed.as_ref()
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to update subscription");

    // when
    app.post_subscriptions(body.into()).await;

    // then assert mock
}

#[tokio::test]
async fn subscribe_returns_a_422_when_email_is_already_confirmed() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = $1
        WHERE email = 'imie.nazwisko@example.com'
        "#,
        SubscriptionStatus::Confirmed.as_ref(),
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to update subscription");

    // when
    let response = app.post_subscriptions(body.into()).await;

    // then
    assert_eq!(
        response.status(),
        422,
        "The API did not return a 422 Unprocessable Entity",
    );
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    sqlx::query!(
        r#"
        ALTER TABLE subscription_tokens
        DROP COLUMN subscription_token
        "#
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // when
    let response = app.post_subscriptions(body.into()).await;

    // then
    assert_eq!(response.status(), 500);
}
