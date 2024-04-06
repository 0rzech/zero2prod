use crate::helpers::TestApp;

#[tokio::test]
async fn login_is_required_to_access_admin_dashboard() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.get_admin_dashboard().await;

    // then
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}
