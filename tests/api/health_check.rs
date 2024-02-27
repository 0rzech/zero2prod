use crate::helpers::TestApp;

#[tokio::test]
async fn health_check_works() {
    // given
    let app = TestApp::spawn().await;

    // when
    let response = app.get_health_check().await;

    // then
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
