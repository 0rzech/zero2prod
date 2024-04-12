use crate::{
    app_state::AppState,
    domain::{SubscriberEmail, SubscriptionStatus},
    utils::{e500, HttpError},
};
use anyhow::Context;
use axum::{extract::State, Json};
use serde::Deserialize;
use sqlx::PgPool;

#[tracing::instrument(name = "Publish newsletter", skip(app_state, body))]
pub(in crate::routes::admin) async fn publish_newsletter(
    State(app_state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<(), HttpError<anyhow::Error>> {
    for subscriber in get_confirmed_subscribers(&app_state.db_pool)
        .await
        .map_err(e500)?
    {
        match subscriber {
            Ok(subscriber) => app_state
                .email_client
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
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

    Ok(())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
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
pub(in crate::routes::admin) struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
