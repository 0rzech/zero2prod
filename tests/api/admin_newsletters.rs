use crate::helpers::{assert_redirect_to, ConfirmationLinks, TestApp};
use serde_json::json;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "newsletter_title": "Newsletter Title",
        "newsletter_html": "<p>Newsletter body as html.</p>",
        "newsletter_text": "Newsletter body as text.",
    });

    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // then
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "newsletter_title": "Newsletter Title",
        "newsletter_html": "<p>Newsletter body as html.</p>",
        "newsletter_text": "Newsletter body as text.",
    });

    create_unfonfirmed_subscriber(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // then
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        (
            json!({
                "newsletter_html": "<p>Newsletter body as html.</p>",
                "newsletter_text": "Newsletter body as text.",
            }),
            "missing title",
        ),
        (
            json!({
                "newsletter_title": "Newsletter Title",
                "newsletter_text": "Newsletter body as text.",
            }),
            "missing html content",
        ),
        (
            json!({
                "newsletter_title": "Newsletter Title",
                "newsletter_html": "<p>Newsletter body as html.</p>",
            }),
            "missing text content",
        ),
    ];

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    for (body, error_message) in test_cases {
        // when
        let response = app.post_publish_newsletter(&body).await;

        // then
        assert_eq!(
            response.status(),
            422,
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn requests_from_anonymous_users_are_redirected_to_login() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "newsletter_title": "Newsletter Title",
        "newsletter_html": "<p>Newsletter body as html.</p>",
        "newsletter_text": "Newsletter body as text.",
    });

    // when
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // then
    assert_redirect_to(&response, "/login");
}

async fn create_unfonfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    let _mock_guard_ = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    app.get_confirmation_links(
        &app.email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap(),
    )
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let links = create_unfonfirmed_subscriber(app).await;
    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
