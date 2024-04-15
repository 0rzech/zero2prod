use crate::helpers::{assert_redirect_to, ConfirmationLinks, TestApp};
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, MockBuilder, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "html_content": "<p>Newsletter body as html.</p>",
        "text_content": "Newsletter body as text.",
        "idempotency_key": Uuid::new_v4(),
    });

    create_confirmed_subscriber(&app).await;
    when_sending_an_email()
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
    assert_redirect_to(&response, "/admin/newsletters");

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "html_content": "<p>Newsletter body as html.</p>",
        "text_content": "Newsletter body as text.",
        "idempotency_key": Uuid::new_v4(),
    });

    create_unconfirmed_subscriber(&app).await;
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
    assert_redirect_to(&response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        (
            json!({
                "html_content": "<p>Newsletter body as html.</p>",
                "text_content": "Newsletter body as text.",
                "idempotency_key": Uuid::new_v4(),
            }),
            "missing title",
        ),
        (
            json!({
                "title": "Newsletter Title",
                "text_content": "Newsletter body as text.",
                "idempotency_key": Uuid::new_v4(),
            }),
            "missing html content",
        ),
        (
            json!({
                "title": "Newsletter Title",
                "html_content": "<p>Newsletter body as html.</p>",
                "idempotency_key": Uuid::new_v4(),
            }),
            "missing text content",
        ),
        (
            json!({
                "title": "Newsletter Title",
                "html_content": "<p>Newsletter body as html.</p>",
                "text_content": "Newsletter body as text.",
            }),
            "missing idempotency key",
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
        "title": "Newsletter Title",
        "html_content": "<p>Newsletter body as html.</p>",
        "text_content": "Newsletter body as text.",
    });

    // when
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // then
    assert_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "html_content": "<p>Newsletter body as html.</p>",
        "text_content": "Newsletter body as text.",
        "idempotency_key": Uuid::new_v4(),
    });

    create_confirmed_subscriber(&app).await;
    app.log_in(&app.test_user.username, &app.test_user.password)
        .await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_newsletter_form_html().await;
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted \
        - emails will go out shortly.</i></p>"
    ));
    app.dispatch_all_pending_emails().await;

    // when
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // then
    assert_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletter_form_html().await;
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted \
        - emails will go out shortly.</i></p>"
    ));
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "html_content": "<p>Newsletter body as html.</p>",
        "text_content": "Newsletter body as text.",
        "idempotency_key": Uuid::new_v4(),
    });

    create_confirmed_subscriber(&app).await;
    app.log_in(&app.test_user.username, &app.test_user.password)
        .await;
    when_sending_an_email()
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // when
    let response1 = app.post_publish_newsletter(&newsletter_request_body);
    let response2 = app.post_publish_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    // then
    assert_redirect_to(&response1, "/admin/newsletters");
    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    app.dispatch_all_pending_emails().await;
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(json!({
        "name": name,
        "email": email,
    }))
    .unwrap();

    let _mock_guard_ = when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body)
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
    let links = create_unconfirmed_subscriber(app).await;
    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}
