#[tokio::test]
async fn health_check_works() {
    // given
    let client = reqwest::Client::new();
    let url = url("health_check");
    spawn_app();

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

fn url(endpoint: &str) -> String {
    format!("http://localhost:8000/{endpoint}")
}

fn spawn_app() {
    tokio::spawn(async move {
        zero2prod::run().await.expect("Failed to run server");
    });
}
