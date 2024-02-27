use crate::helpers::{spawn_app, url};

#[tokio::test]
async fn health_check_works() {
    // given
    let client = reqwest::Client::new();
    let app = spawn_app().await;
    let url = url(app.address, "health_check");

    // when
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute request");

    // then
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
