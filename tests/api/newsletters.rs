use crate::helpers::{ConfirmationLinks, TestApp};
use serde_json::json;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text.",
            "html": "<p>Newsletter body as html.</p>",
        }
    });

    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // when
    let response = app.post_newsletters(&newsletter_request_body).await;

    // then
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text.",
            "html": "<p>Newsletter body as html.</p>",
        }
    });

    create_unfonfirmed_subscriber(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // when
    let response = app.post_newsletters(&newsletter_request_body).await;

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
                "content": {
                    "text": "Newsletter body as plain text.",
                    "html": "<p>Newsletter body as html.</p>",
                }
            }),
            "missing title",
        ),
        (
            json!({
                "title": "Newsletter Title",
            }),
            "missing content",
        ),
    ];

    for (body, error_message) in test_cases {
        // when
        let response = app.post_newsletters(&body).await;

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
async fn requests_missing_authorization_are_rejected() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text.",
            "html": "<p>Newsletter body as html.</p>",
        }
    });

    // when
    let response = app.post_newsletters_no_auth(&newsletter_request_body).await;

    // then
    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text.",
            "html": "<p>Newsletter body as html.</p>",
        }
    });
    let username = Uuid::new_v4().to_string();

    // when
    let response = app
        .post_newsletters_with_credentials(
            &newsletter_request_body,
            &username,
            &app.test_user.password,
        )
        .await;

    // then
    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    // given
    let app = TestApp::spawn().await;
    let newsletter_request_body = json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text.",
            "html": "<p>Newsletter body as html.</p>",
        }
    });
    let password = Uuid::new_v4().to_string();

    // when
    let response = app
        .post_newsletters_with_credentials(
            &newsletter_request_body,
            &app.test_user.username,
            &password,
        )
        .await;

    // then
    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
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
