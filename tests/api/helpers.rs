use once_cell::sync::Lazy;
use reqwest::{Client, Response};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let name = "test";
    let default_env_filter = "info";
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(name.into(), default_env_filter.into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(name.into(), default_env_filter.into(), std::io::sink);
        init_subscriber(subscriber);
    }
});

static FAILED_TO_EXECUTE_REQUEST: &'static str = "Failed to execute request";

pub struct TestApp {
    pub address: SocketAddr,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    client: Client,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Lazy::force(&TRACING);

        let mut config = get_configuration().expect("Failed to read configuration");
        config.database.database_name = Uuid::new_v4().to_string();
        config.application.port = 0;

        let db_pool = configure_database(&config.database).await;
        let email_server = MockServer::start().await;
        config.email_client.base_url = email_server.uri();

        let app = Application::build(config).await;
        let address = app.local_addr();

        tokio::spawn(app.run_until_stopped());

        Self {
            address,
            db_pool,
            email_server,
            client: Client::new(),
        }
    }

    pub async fn get_health_check(&self) -> Response {
        self.client
            .get(self.url("/health_check"))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    pub async fn post_subscriptions(&self, body: String) -> Response {
        self.client
            .post(self.url("/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    fn url(&self, endpoint: &str) -> String {
        format!("http://{}{endpoint}", self.address)
    }
}

async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
    let mut conn = PgConnection::connect_with(&configuration.without_db())
        .await
        .expect("Failed to connect to Postgres");

    conn.execute(format!(r#"CREATE DATABASE "{}";"#, configuration.database_name).as_str())
        .await
        .expect("Failed to create database");

    let pool = get_connection_pool(&configuration);

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate database");

    pool
}
