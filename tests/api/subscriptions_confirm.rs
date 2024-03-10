use crate::helpers::TestApp;
use claims::assert_some_eq;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.confirm_subscription().await;

    // then
    assert_eq!(response.status().as_u16(), 400);
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

    let links = app.get_confirmation_links_from_email_request().await;
    assert_some_eq!(links.html.host_str(), "localhost");

    // when
    let response = reqwest::get(links.html).await.unwrap();

    // then
    assert_eq!(response.status().as_u16(), 200);
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
    let links = app.get_confirmation_links_from_email_request().await;

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
