use crate::{
    app_state::AppState,
    configuration::{ApplicationSettings, DatabaseSettings, Settings},
    email_client::EmailClient,
    request_id::RequestUuid,
    routes::{health_check, home, login, newsletters, subscriptions, subscriptions_confirm},
    telemetry::request_span,
};
use anyhow::anyhow;
use axum::{http::Uri, serve::Serve, Router};
use axum_extra::extract::cookie::Key;
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, str::FromStr};
use time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    fred::{
        clients::RedisPool,
        interfaces::ClientLike,
        types::{ConnectHandle, RedisConfig},
    },
    RedisStore,
};
use tracing::Level;

pub struct Application {
    local_addr: SocketAddr,
    server: Serve<Router, Router>,
    redis_conn: ConnectHandle,
}

impl Application {
    pub async fn build(config: Settings) -> Application {
        let address = format!("{}:{}", config.application.host, config.application.port);

        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to open listener");

        let db_pool = get_pg_connection_pool(&config.database);

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

        let (redis_pool, redis_conn) = get_redis_connection_pool(&config.application).await;

        let local_addr = listener
            .local_addr()
            .expect("Failed to get local address from the listener");

        let server = run(
            listener,
            db_pool,
            email_client,
            config.application.base_url,
            config.application.hmac_secret,
            redis_pool,
        )
        .await;

        Self {
            local_addr,
            server,
            redis_conn,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn run_until_stopped(self) -> Result<(), anyhow::Error> {
        tracing::info!("Listening on {}", self.local_addr);
        self.server.await?;
        self.redis_conn.await?.map_err(|e| anyhow!(e))
    }
}

pub fn get_pg_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.with_db())
}

async fn get_redis_connection_pool(config: &ApplicationSettings) -> (RedisPool, ConnectHandle) {
    let config = RedisConfig::from_url(config.redis_uri.expose_secret())
        .expect("Failed to create redis config");
    let pool = RedisPool::new(config, None, None, None, 6).expect("Failed to create redis pool");
    let conn = pool.connect();

    pool.wait_for_connect()
        .await
        .expect("Failed to connect redis clients");

    (pool, conn)
}

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_pool: RedisPool,
) -> Serve<Router, Router> {
    let key = Key::from(hmac_secret.expose_secret().as_bytes());

    let app_state = AppState {
        db_pool,
        email_client,
        base_url: Uri::from_str(&base_url).expect("Failed to parse base url"),
        hmac_secret: key.clone(),
    };

    let app = Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .merge(subscriptions_confirm::router())
        .merge(newsletters::router())
        .merge(home::router())
        .merge(login::router())
        .with_state(app_state)
        .layer(
            SessionManagerLayer::new(RedisStore::new(redis_pool))
                .with_expiry(Expiry::OnInactivity(Duration::minutes(10)))
                .with_private(key),
        )
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
