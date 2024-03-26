use crate::{
    app_state::AppState,
    authentication::{validate_credentials, AuthError, Credentials},
    domain::{SubscriberEmail, SubscriptionStatus},
};
use anyhow::Context;
use axum::{
    extract::State,
    http::{header::WWW_AUTHENTICATE, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::Engine;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

pub fn router() -> Router<AppState> {
    Router::new().route("/newsletters", post(publish_newsletter))
}

#[tracing::instrument(
    name = "Publish newsletter",
    skip(headers, app_state, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
async fn publish_newsletter(
    headers: HeaderMap,
    State(app_state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(&app_state.db_pool, credentials)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    for subscriber in get_confirmed_subscribers(&app_state.db_pool).await? {
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

#[tracing::instrument(name = "Extract basic auth credentials", skip(headers))]
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let base64encoded_segment = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF-8 string")?
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF-8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
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
struct BodyData {
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

#[derive(Debug, thiserror::Error)]
enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self);

        match self {
            Self::AuthError(_) => {
                let mut headers = HeaderMap::new();
                let header_value = HeaderValue::from_static(r#"Basic realm="publish""#);
                headers.insert(WWW_AUTHENTICATE, header_value);
                (StatusCode::UNAUTHORIZED, headers).into_response()
            }
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
