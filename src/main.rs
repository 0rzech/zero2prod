use sqlx::PgPool;
use tokio::net::TcpListener;
use zero2prod::{
    configuration::get_configuration,
    email_client::EmailClient,
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

    let pool = PgPool::connect_lazy_with(config.database.with_db());

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

    run(listener, pool, email_client).await
}
