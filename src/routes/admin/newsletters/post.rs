use crate::{
    app_state::AppState,
    authentication::extract::SessionUserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e422, e500, HttpError},
};
use anyhow::Context;
use askama_axum::IntoResponse;
use axum::{body::Body, extract::State, http::Response, response::Redirect, Form};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

#[tracing::instrument(skip_all, fields(user_id=%user_id))]
pub(in crate::routes::admin) async fn publish_newsletter(
    State(app_state): State<AppState>,
    SessionUserId(user_id): SessionUserId,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response<Body>, HttpError<anyhow::Error>> {
    let idempotency_key: IdempotencyKey = form.idempotency_key.try_into().map_err(e422)?;

    let mut transaction = match try_processing(&app_state.db_pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message(messages);
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(
        &mut transaction,
        &form.title,
        &form.text_content,
        &form.html_content,
    )
    .await
    .context("Failed to store newsletter issue details")
    .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    success_message(messages);

    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(transaction, &idempotency_key, user_id, response).await?;

    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    );

    transaction.execute(query).await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    );

    transaction.execute(query).await?;

    Ok(())
}

#[derive(Deserialize)]
pub(in crate::routes::admin) struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

fn success_message(messages: Messages) {
    messages.info(
        "The newsletter issue has been accepted \
        - emails will go out shortly.",
    );
}
