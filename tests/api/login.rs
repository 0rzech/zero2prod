use crate::helpers::{assert_redirect_to, TestApp};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn successful_login_redirects_to_admin_dashboard() {
    // given
    let app = TestApp::spawn().await;
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    // when
    let response = app.post_login(&login_body).await;

    // then
    assert_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome, {}!", app.test_user.username)));
}

#[tokio::test]
async fn an_error_flash_message_is_sent_on_failure() {
    // given
    let app = TestApp::spawn().await;
    let login_body = json!({
        "username": Uuid::new_v4().to_string(),
        "password": Uuid::new_v4().to_string(),
    });

    // when
    let response = app.post_login(&login_body).await;

    // then
    assert_redirect_to(&response, "/login");
}

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // given
    let app = TestApp::spawn().await;
    let login_body = json!({
        "username": Uuid::new_v4().to_string(),
        "password": Uuid::new_v4().to_string(),
    });

    // when
    app.post_login(&login_body).await;

    // then
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
}
