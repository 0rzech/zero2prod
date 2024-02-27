use crate::{
    app_state::AppState,
    email_client::EmailClient,
    request_id::RequestUuid,
    routes::{health_check, subscriptions},
    telemetry::request_span,
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

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<(), std::io::Error> {
    let app_state = AppState {
        db_pool,
        email_client,
    };

    let app = Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .with_state(app_state)
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
