use crate::{
    app_state::AppState,
    domain::{SubscriptionStatus, SubscriptionToken},
};
use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use secrecy::ExposeSecret;
use serde::Deserialize;
use sqlx::{Executor, Postgres, Row, Transaction};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions/confirm", get(confirm))
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(app_state, parameters))]
async fn confirm(
    State(app_state): State<AppState>,
    Query(parameters): Query<Parameters>,
) -> Result<(), SubscriptionConfirmationError> {
    let mut transaction = app_state
        .db_pool
        .begin()
        .await
        .context("Failed to begin transaction")?;

    let subscription_token = SubscriptionToken::parse(parameters.subscription_token)
        .map_err(SubscriptionConfirmationError::InvalidTokenFormat)?;

    let subscriber_id =
        match get_subscriber_id_from_token(&mut transaction, &subscription_token).await? {
            Some(id) => id,
            None => return Err(SubscriptionConfirmationError::UnauthorizedToken),
        };

    confirm_subscriber(&mut transaction, subscriber_id).await?;
    delete_confirmation_tokens(&mut transaction, subscriber_id).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok(())
}

#[derive(Deserialize)]
struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(transaction, subscription_token)
)]
async fn get_subscriber_id_from_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, anyhow::Error> {
    let query = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token.expose_secret(),
    );

    let subscriber_id = match transaction
        .fetch_optional(query)
        .await
        .context("Failed fetch subscriber id")?
    {
        Some(row) => row
            .try_get("subscriber_id")
            .context("Failed to instantiate subscriber_id")?,
        _ => None,
    };

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed",
    skip(transaction, subscriber_id)
)]
async fn confirm_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        UPDATE subscriptions SET status = $1
        WHERE id = $2
        "#,
        SubscriptionStatus::Confirmed.as_ref(),
        subscriber_id,
    );

    transaction
        .execute(query)
        .await
        .context("Failed to update subscription status")?;

    Ok(())
}

#[tracing::instrument(
    name = "Delete subscription confirmation tokens",
    skip(transaction, subscriber_id)
)]
async fn delete_confirmation_tokens(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM subscription_tokens
        WHERE subscriber_id = $1
        "#,
        subscriber_id
    );

    transaction
        .execute(query)
        .await
        .context("Failed to delete subscription confirmation tokens")?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum SubscriptionConfirmationError {
    #[error("{0}")]
    InvalidTokenFormat(String),
    #[error("Token is not authorized")]
    UnauthorizedToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for SubscriptionConfirmationError {
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self);

        match self {
            Self::InvalidTokenFormat(_) => {
                (StatusCode::BAD_REQUEST, self.to_string()).into_response()
            }
            Self::UnauthorizedToken => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
