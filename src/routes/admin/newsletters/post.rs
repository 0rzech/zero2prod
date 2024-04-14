use crate::{
    app_state::AppState,
    authentication::extract::SessionUserId,
    domain::{SubscriberEmail, SubscriptionStatus},
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e422, e500, HttpError},
};
use anyhow::Context;
use askama_axum::IntoResponse;
use axum::{body::Body, extract::State, http::Response, response::Redirect, Form};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgPool;

#[tracing::instrument(skip(app_state, user_id, messages, form))]
pub(in crate::routes::admin) async fn publish_newsletter(
    State(app_state): State<AppState>,
    SessionUserId(user_id): SessionUserId,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response<Body>, HttpError<anyhow::Error>> {
    let flash_success = || messages.info("Newsletter sent!");
    let idempotency_key: IdempotencyKey = form.idempotency_key.try_into().map_err(e422)?;

    let transaction = match try_processing(&app_state.db_pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            flash_success();
            return Ok(saved_response);
        }
    };

    for subscriber in get_confirmed_subscribers(&app_state.db_pool)
        .await
        .map_err(e500)?
    {
        match subscriber {
            Ok(subscriber) => app_state
                .email_client
                .send_email(
                    &subscriber.email,
                    &form.title,
                    &form.html_content,
                    &form.text_content,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter issue to {}", subscriber.email)
                })?,
            Err(e) => tracing::warn!(
                e.cause_chain = ?e,
                "Skipping a confirmed subscriber. \
                Ther stored contact details are invalid"
            ),
        }
    }

    flash_success();

    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(transaction, &idempotency_key, user_id, response).await?;

    Ok(response)
}

#[tracing::instrument(skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = $1
        "#,
        SubscriptionStatus::Confirmed.as_ref(),
    )
    .fetch_all(db_pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|row| match SubscriberEmail::parse(row.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(e) => Err(anyhow::anyhow!(e)),
            })
            .collect()
    })?;

    Ok(subscribers)
}

#[derive(Deserialize)]
pub(in crate::routes::admin) struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
