use crate::routes::{health_check, subscriptions};
use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;

pub async fn run(listener: TcpListener, db_pool: PgPool) -> Result<(), std::io::Error> {
    let app = Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .with_state(db_pool);

    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}
