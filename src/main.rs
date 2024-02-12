#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    zero2prod::run(tokio::net::TcpListener::bind("127.0.0.1:8000").await?).await
}
