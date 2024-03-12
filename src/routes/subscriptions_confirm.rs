use crate::{
    app_state::AppState,
    domain::{SubscriptionStatus, SubscriptionToken},
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
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
) -> StatusCode {
    let mut transaction = match app_state.db_pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let subscription_token = match SubscriptionToken::parse(parameters.subscription_token) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(e);
            return StatusCode::BAD_REQUEST;
        }
    };

    let subscriber_id =
        match get_subscriber_id_from_token(&mut transaction, &subscription_token).await {
            Ok(Some(id)) => id,
            Ok(None) => return StatusCode::UNAUTHORIZED,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };

    if confirm_subscriber(&mut transaction, subscriber_id)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if delete_confirmation_tokens(&mut transaction, subscriber_id)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(e) = transaction.commit().await {
        tracing::error!("Failed to commit transaction: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[derive(Deserialize)]
struct Parameters {
    #[allow(dead_code)]
    subscription_token: String,
}

#[tracing::instrument(
    name = "Getting subscriber_id from token",
    skip(transaction, subscription_token)
)]
async fn get_subscriber_id_from_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let query = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token.expose_secret(),
    );

    let result = transaction.fetch_optional(query).await.map_err(|e| {
        tracing::error!("Failed fetch subscriber id: {:?}", e);
        e
    })?;

    let subscriber_id = match result {
        Some(row) => row.try_get("subscriber_id").map_err(|e| {
            tracing::error!("Failed to instantiate subscriber_id: {:?}", e);
            e
        })?,
        _ => None,
    };

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Marking subscriber as confirmed",
    skip(transaction, subscriber_id)
)]
async fn confirm_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        UPDATE subscriptions SET status = $1
        WHERE id = $2
        "#,
        SubscriptionStatus::Confirmed.as_ref(),
        subscriber_id,
    );

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Deleting subscribe confirmation tokens",
    skip(transaction, subscriber_id)
)]
async fn delete_confirmation_tokens(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM subscription_tokens
        WHERE subscriber_id = $1
        "#,
        subscriber_id
    );

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to delete subscription confirmation tokens: {:?}", e);
        e
    })?;

    Ok(())
}
