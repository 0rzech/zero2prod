use crate::helpers::{assert_redirect_to, TestApp};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn changing_password_works() {
    // given
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();
    let body = json!({
        "current_password": &app.test_user.password,
        "new_password": new_password,
        "new_password_check": new_password,
    });

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_change_password(&body).await;
    assert_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = app.post_logout().await;
    assert_redirect_to(&response, "/login");

    let response = app
        .post_login(&json!({
           "username": &app.test_user.username,
           "password": new_password,
        }))
        .await;

    // then
    assert_redirect_to(&response, "/admin/dashboard");
}

#[tokio::test]
async fn login_is_required_to_access_admin_password() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.get_change_password_form().await;

    // then
    assert_redirect_to(&response, "/login");
}

#[tokio::test]
async fn login_is_required_to_change_password() {
    // given
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();
    let body = json!({
        "current_password": Uuid::new_v4(),
        "new_password": new_password,
        "new_password_check": new_password,
    });

    // when
    let response = app.post_change_password(&body).await;

    // then
    assert_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_check_is_validated() {
    // given
    let app = TestApp::spawn().await;
    let body = json!({
        "current_password": &app.test_user.password,
        "new_password": Uuid::new_v4(),
        "new_password_check": Uuid::new_v4(),
    });

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_change_password(&body).await;
    assert_redirect_to(&response, "/admin/password");

    // then
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You have entered two different new passwords - \
        the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_is_validated() {
    // given
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();
    let body = json!({
        "current_password": Uuid::new_v4(),
        "new_password": new_password,
        "new_password_check": new_password,
    });

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_change_password(&body).await;
    assert_redirect_to(&response, "/admin/password");

    // then
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn too_short_password_is_rejected() {
    // given
    let app = TestApp::spawn().await;
    let new_password = "a".repeat(11);
    let body = json!({
        "current_password": &app.test_user.password,
        "new_password": new_password,
        "new_password_check": new_password,
    });

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_change_password(&body).await;
    assert_redirect_to(&response, "/admin/password");

    // then
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Password must be at least 12 characters long.</i></p>"));
}

#[tokio::test]
async fn too_long_password_is_rejected() {
    // given
    let app = TestApp::spawn().await;
    let new_password = "a".repeat(129);
    let body = json!({
        "current_password": &app.test_user.password,
        "new_password": new_password,
        "new_password_check": new_password,
    });

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    // when
    let response = app.post_change_password(&body).await;
    assert_redirect_to(&response, "/admin/password");

    // then
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Passwords must be at most 128 characters long.</i></p>"));
}

#[tokio::test]
async fn logout_clears_session() {
    // given
    let app = TestApp::spawn().await;

    let response = app
        .post_login(&json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome, {}!", &app.test_user.username)));

    // when
    let response = app.post_logout().await;

    // then
    assert_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    let response = app.get_admin_dashboard().await;
    assert_redirect_to(&response, "/login");
}
