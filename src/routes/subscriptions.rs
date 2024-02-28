use crate::{
    app_state::AppState,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
};
use axum::{extract::State, http::StatusCode, routing::post, Form, Router};
use serde::Deserialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions", post(subscribe))
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, app_state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
async fn subscribe(State(app_state): State<AppState>, Form(form): Form<FormData>) -> StatusCode {
    let new_subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::info!(e);
            return StatusCode::BAD_REQUEST;
        }
    };

    match insert_subscriber(&new_subscriber, &app_state.db_pool).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, db_pool)
)]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    db_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        OffsetDateTime::now_utc(),
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    Ok(())
}

#[derive(Deserialize)]
struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(value.email)?;
        let name = SubscriberName::parse(value.name)?;
        Ok(NewSubscriber { email, name })
    }
}
