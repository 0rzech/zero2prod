use std::net::SocketAddr;

#[tokio::test]
async fn health_check_works() {
    // given
    let client = reqwest::Client::new();
    let addr = spawn_app().await;
    let url = url(addr, "health_check");

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

async fn spawn_app() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind address");
    let addr = listener.local_addr().expect("Failed to get local address");

    tokio::spawn(async move {
        zero2prod::run(listener)
            .await
            .expect("Failed to run server");
    });

    addr
}

fn url(addr: SocketAddr, endpoint: &str) -> String {
    format!("http://{}/{}", addr, endpoint)
}
