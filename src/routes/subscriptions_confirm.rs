use crate::app_state::AppState;
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
        &parameters.subscription_token,
        &app_state.db_pool,
    )
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::UNAUTHORIZED,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if confirm_subscriber(subscriber_id, &app_state.db_pool)
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
    skip(subscription_token, db_pool)
)]
async fn get_subscriber_id_from_token(
    subscription_token: &str,
    db_pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Marking subscriber as confirmed", skip(subscriber_id, db_pool))]
async fn confirm_subscriber(subscriber_id: Uuid, db_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id,
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
