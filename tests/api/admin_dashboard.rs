use crate::helpers::{assert_redirect_to, TestApp};

#[tokio::test]
async fn login_is_required_to_access_admin_dashboard() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.get_admin_dashboard().await;

    // then
    assert_redirect_to(&response, "/login");
}
