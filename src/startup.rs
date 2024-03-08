use crate::{
    app_state::AppState,
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    request_id::RequestUuid,
    routes::{health_check, subscriptions, subscriptions_confirm},
    telemetry::request_span,
};
use axum::{http::Uri, serve::Serve, Router};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, str::FromStr};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::Level;

pub struct Application {
    local_addr: SocketAddr,
    server: Serve<Router, Router>,
}

impl Application {
    pub async fn build(config: Settings) -> Application {
        let address = format!("{}:{}", config.application.host, config.application.port);

        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to open listener");

        let db_pool = get_connection_pool(&config.database);

        let sender_email = config
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = config.email_client.timeout();

        let email_client = EmailClient::new(
            config.email_client.base_url,
            sender_email,
            config.email_client.authorization_token,
            timeout,
        );

        let local_addr = listener
            .local_addr()
            .expect("Failed to get local address from the listener");

        let server = run(listener, db_pool, email_client, config.application.base_url).await;

        Self { local_addr, server }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        tracing::info!("Listening on {}", self.local_addr);
        self.server.await
    }
}

pub fn get_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.with_db())
}

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Serve<Router, Router> {
    let app_state = AppState {
        db_pool,
        email_client,
        base_url: Uri::from_str(&base_url).expect("Failed to parse base url"),
    };

    let app = Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .merge(subscriptions_confirm::router())
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

    axum::serve(listener, app)
}
