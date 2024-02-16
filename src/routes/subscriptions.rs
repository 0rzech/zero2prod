use axum::{extract::State, http::StatusCode, routing::post, Form, Router};
use serde::Deserialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<PgPool> {
    Router::new().route("/subscriptions", post(subscribe))
}

async fn subscribe(State(db_pool): State<PgPool>, Form(form): Form<Subscription>) -> StatusCode {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        OffsetDateTime::now_utc(),
    )
    .execute(&db_pool)
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Failed to execute query: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[derive(Deserialize)]
struct Subscription {
    name: String,
    email: String,
}
