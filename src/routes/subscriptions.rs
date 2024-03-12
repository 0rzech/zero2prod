use crate::{
    app_state::AppState,
    domain::{
        NewSubscriber, SubscriberEmail, SubscriberName, Subscription, SubscriptionStatus,
        SubscriptionToken,
    },
    email_client::EmailClient,
};
use axum::{
    extract::State,
    http::{StatusCode, Uri},
    routing::post,
    Form, Router,
};
use reqwest::Error;
use secrecy::ExposeSecret;
use serde::Deserialize;
use sqlx::{Executor, FromRow, Postgres, Transaction};
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
    let new_subscriber: NewSubscriber = match form.try_into() {
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

    let subscription = match get_subscription(&mut transaction, &new_subscriber.email).await {
        Ok(subscription) => subscription,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let subscriber_id = if let Some(subscription) = subscription {
        if subscription.status == SubscriptionStatus::Confirmed {
            return StatusCode::UNPROCESSABLE_ENTITY;
        }
        subscription.id
    } else {
        match insert_subscriber(&mut transaction, &new_subscriber).await {
            Ok(subscriber_id) => subscriber_id,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        }
    };

    let subscription_token = SubscriptionToken::generate();

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
    name = "Getting subscriber details from the database",
    skip(transaction, email)
)]
async fn get_subscription(
    transaction: &mut Transaction<'_, Postgres>,
    email: &SubscriberEmail,
) -> Result<Option<Subscription>, sqlx::Error> {
    let query = sqlx::query!(
        r#"
        SELECT * FROM subscriptions
        WHERE email = $1
        "#,
        email.as_ref()
    );

    let subscription = match transaction.fetch_optional(query).await {
        Ok(Some(row)) => Subscription::from_row(&row)
            .map_err(|e| {
                tracing::error!("Failed to instantiate subscription: {:?}", e);
                e
            })
            .map(Some),
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("Failed to get subscriber: {:?}", e);
            Err(e)
        }
    }?;

    Ok(subscription)
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
        VALUES ($1, $2, $3, $4, $5)
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        OffsetDateTime::now_utc(),
        SubscriptionStatus::PendingConfirmation.as_ref(),
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
    subscription_token: &SubscriptionToken,
) -> Result<(), Error> {
    let confirmation_link = format!(
        "{base_url}subscriptions/confirm?subscription_token={}",
        subscription_token.expose_secret()
    );
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

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(transaction, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token.expose_secret(),
        subscriber_id
    );

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
