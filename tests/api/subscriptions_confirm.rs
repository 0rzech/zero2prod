use crate::helpers::TestApp;
use claims::assert_some_eq;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmation_without_token_is_rejected_with_a_400() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.confirm_subscription_without_token().await;

    // then
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn confirmation_with_empty_token_is_rejected_with_a_400() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.confirm_subscription("").await;

    // then
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn confirmation_with_invalid_token_is_rejected_with_a_400() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.confirm_subscription("a").await;

    // then
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn confirmation_with_invalid_token_returns_validation_error_description() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.confirm_subscription("a").await;
    let body = response.text().await.unwrap();

    // then
    assert!(
        body.to_lowercase().contains("invalid"),
        "'{body}' response body did not contain 'invalid' word"
    );
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200_if_called() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let links = app.get_confirmation_links(&app.email_server.received_requests().await.unwrap()[0]);
    assert_some_eq!(links.html.host_str(), "localhost");

    // when
    let response = reqwest::get(links.html).await.unwrap();

    // then
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let links = app.get_confirmation_links(&app.email_server.received_requests().await.unwrap()[0]);

    // when
    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // then
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "imie.nazwisko@example.com");
    assert_eq!(saved.name, "Imię Nazwisko");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn subsequent_clicks_on_the_confirmation_link_are_rejected_with_a_401() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let links = app.get_confirmation_links(&app.email_server.received_requests().await.unwrap()[0]);
    assert_some_eq!(links.html.host_str(), "localhost");

    // when
    reqwest::get(links.html.clone()).await.unwrap();
    let response = reqwest::get(links.html).await.unwrap();

    // then
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_deletes_subscription_tokens() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let links = app.get_confirmation_links(&app.email_server.received_requests().await.unwrap()[0]);
    assert_some_eq!(links.html.host_str(), "localhost");

    // when
    reqwest::get(links.html).await.unwrap();

    // then
    let result = sqlx::query!("SELECT * FROM subscription_tokens")
        .fetch_all(&app.db_pool)
        .await
        .expect("Failed to fetch subscription_tokens");

    assert_eq!(result.len(), 0);
}
