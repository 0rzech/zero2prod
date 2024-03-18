use crate::{
    app_state::AppState,
    domain::{
        NewSubscriber, SubscriberEmail, SubscriberName, Subscription, SubscriptionStatus,
        SubscriptionToken,
    },
    email_client::EmailClient,
};
use anyhow::Context;
use askama::Template;
use axum::{
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::post,
    Form, Router,
};
use secrecy::ExposeSecret;
use serde::Deserialize;
use sqlx::{Executor, FromRow, Postgres, Transaction};
use std::fmt::Debug;
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
async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<(), SubscribeError> {
    let new_subscriber: NewSubscriber = form.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = app_state
        .db_pool
        .begin()
        .await
        .context("Failed to begin transaction")?;

    let subscriber_id = match get_subscription(&mut transaction, &new_subscriber.email).await? {
        Some(Subscription {
            status: SubscriptionStatus::PendingConfirmation,
            id,
            ..
        }) => id,
        Some(_) => return Err(SubscribeError::SubscriptionAlreadyConfirmed),
        None => insert_subscriber(&mut transaction, &new_subscriber).await?,
    };

    let subscription_token = SubscriptionToken::generate();

    store_token(&mut transaction, subscriber_id, &subscription_token).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    send_confirmation_email(
        &app_state.email_client,
        new_subscriber,
        &app_state.base_url,
        &subscription_token,
    )
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Getting subscriber details from the database",
    skip(transaction, email)
)]
async fn get_subscription(
    transaction: &mut Transaction<'_, Postgres>,
    email: &SubscriberEmail,
) -> Result<Option<Subscription>, anyhow::Error> {
    let query = sqlx::query!(
        r#"
        SELECT * FROM subscriptions
        WHERE email = $1
        "#,
        email.as_ref()
    );

    let subscription = match transaction
        .fetch_optional(query)
        .await
        .context("Failed to fetch subscription")?
    {
        Some(row) => Subscription::from_row(&row)
            .map(Some)
            .context("Failed to instantiate subscription")?,
        _ => None,
    };

    Ok(subscription)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, anyhow::Error> {
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

    transaction
        .execute(query)
        .await
        .context("Failed to insert new subscriber")?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(transaction, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token.expose_secret(),
        subscriber_id
    );

    transaction
        .execute(query)
        .await
        .context("Failed to store token")?;

    Ok(())
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
) -> Result<(), anyhow::Error> {
    let link = format!(
        "{base_url}subscriptions/confirm?subscription_token={}",
        subscription_token.expose_secret()
    );

    let html_body = HtmlBodyTemplate {
        confirmation_link: &link,
    }
    .render()
    .context("Failed to render html template")?;

    let plain_body = PlainTextBodyTemplate {
        confirmation_link: &link,
    }
    .render()
    .context("Failed to render plain text template")?;

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .context("Failed to execute request")
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

#[derive(Template)]
#[template(path = "welcome.html")]
struct HtmlBodyTemplate<'a> {
    confirmation_link: &'a str,
}

#[derive(Template)]
#[template(path = "welcome.txt")]
struct PlainTextBodyTemplate<'a> {
    confirmation_link: &'a str,
}

#[derive(Debug, thiserror::Error)]
enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Subscription has been confirmed already")]
    SubscriptionAlreadyConfirmed,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self);

        match self {
            Self::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
            Self::SubscriptionAlreadyConfirmed => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
            }
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
