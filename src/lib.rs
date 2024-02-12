use axum::{http::StatusCode, routing::get, Router};

pub async fn run() -> Result<(), std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;

    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
