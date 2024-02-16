use sqlx::PgPool;
use tokio::net::TcpListener;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = get_configuration().expect("Failed to read configuration.");
    let address = format!("{}:{}", config.application_host, config.application_port);
    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to open listener.");
    let pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    run(listener, pool).await
}
