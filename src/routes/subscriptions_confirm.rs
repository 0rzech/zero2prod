use crate::{app_state::AppState, domain::SubscriptionStatus};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions/confirm", get(confirm))
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(app_state, parameters))]
async fn confirm(
    State(app_state): State<AppState>,
    Query(parameters): Query<Parameters>,
) -> StatusCode {
    let subscriber_id = match get_subscriber_id_from_token(
        &app_state.db_pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::UNAUTHORIZED,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if confirm_subscriber(&app_state.db_pool, subscriber_id)
        .await
        .is_err()
    {
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
    skip(pool, subscription_token)
)]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Marking subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = $1
        WHERE id = $2
        "#,
        SubscriptionStatus::Confirmed.as_ref(),
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
