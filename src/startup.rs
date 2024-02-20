use crate::{
    routes::{health_check, subscriptions},
    telemetry::{request_span, RequestUuid},
};
use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::Level;

pub async fn run(listener: TcpListener, db_pool: PgPool) -> Result<(), std::io::Error> {
    let app = Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .with_state(db_pool)
        .layer(
            ServiceBuilder::new()
                .set_x_request_id(RequestUuid)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(request_span)
                        .on_request(DefaultOnRequest::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .propagate_x_request_id(),
        );

    tracing::info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}
