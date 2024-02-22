use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration");
    let address = format!("{}:{}", config.application.host, config.application.port);

    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to open listener");

    let pool = PgPool::connect_lazy(config.database.connection_string().expose_secret())
        .expect("Failed to connect to Postgres");

    run(listener, pool).await
}
