use crate::{
    app_state::AppState,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};
use axum::{
    extract::State,
    http::{StatusCode, Uri},
    routing::post,
    Form, Router,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Error;
use serde::Deserialize;
use sqlx::{Executor, Postgres, Transaction};
use std::iter::repeat_with;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions", post(subscribe))
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(app_state, form),
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

    let mut transaction = match app_state.db_pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let subscription_token = generate_subscription_token();

    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(e) = transaction.commit().await {
        tracing::error!("Failed to commit transaction: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_confirmation_email(
        &app_state.email_client,
        new_subscriber,
        &app_state.base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        OffsetDateTime::now_utc(),
    );

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Sending confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &Uri,
    subscription_token: &str,
) -> Result<(), Error> {
    let confirmation_link =
        format!("{base_url}subscriptions/confirm?subscription_token={subscription_token}");
    let html_body = format!(
        "Welcome to our newsletter!<br/>\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
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

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(transaction, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    );

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
