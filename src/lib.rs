use axum::{http::StatusCode, routing::get, Router};

pub async fn run(listener: tokio::net::TcpListener) -> Result<(), std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
