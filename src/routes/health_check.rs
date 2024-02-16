use axum::{http::StatusCode, routing::get, Router};
use sqlx::PgPool;

pub fn router() -> Router<PgPool> {
    Router::new().route("/health_check", get(health_check))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
