use crate::{
    app_state::AppState,
    domain::{SubscriberEmail, SubscriptionStatus},
    utils::{e500, HttpError},
};
use anyhow::Context;
use axum::{extract::State, response::Redirect, Form};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgPool;

#[tracing::instrument(skip(app_state, form))]
pub(in crate::routes::admin) async fn publish_newsletter(
    State(app_state): State<AppState>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Redirect, HttpError<anyhow::Error>> {
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

    messages.info("Newsletter sent!");

    Ok(Redirect::to("/admin/newsletters"))
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
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
