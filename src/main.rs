use axum::{extract::Path, response::Html, routing::get, Router};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(greet))
        .route("/:name", get(greet));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;

    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

async fn greet(name: Option<Path<String>>) -> Html<String> {
    let name = match name {
        Some(Path(name)) => name,
        None => "World".to_string(),
    };

    Html(format!("Hello, {}!", name))
}
