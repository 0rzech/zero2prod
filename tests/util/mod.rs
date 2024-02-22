use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
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

pub struct TestApp {
    pub address: SocketAddr,
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let mut config = get_configuration().expect("Failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();

    let listener = tokio::net::TcpListener::bind("localhost:0")
        .await
        .expect("Failed to bind address");

    let app = TestApp {
        address: listener.local_addr().expect("Failed to get local address"),
        db_pool: configure_database(&config.database).await,
    };

    let pool = app.db_pool.clone();
    tokio::spawn(async move {
        run(listener, pool).await.expect("Failed to run server");
    });

    app
}

pub fn url(addr: SocketAddr, endpoint: &str) -> String {
    format!("http://{}/{}", addr, endpoint)
}

async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
    let mut conn = PgConnection::connect_with(&configuration.without_db())
        .await
        .expect("Failed to connect to Postgres");

    conn.execute(format!(r#"CREATE DATABASE "{}";"#, configuration.database_name).as_str())
        .await
        .expect("Failed to create database");

    let pool = PgPool::connect_with(configuration.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate database");

    pool
}
