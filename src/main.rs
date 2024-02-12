#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    zero2prod::run().await
}
