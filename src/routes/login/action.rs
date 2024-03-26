use crate::{
    app_state::AppState,
    authentication::{validate_credentials, AuthError, Credentials},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use urlencoding::Encoded;

#[tracing::instrument(
    skip(app_state, form),
    fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub(super) async fn login(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<Redirect, LoginError> {
    tracing::Span::current().record("username", &tracing::field::display(&form.username));

    let user_id = validate_credentials(
        &app_state.db_pool,
        Credentials {
            username: form.username,
            password: form.password,
        },
    )
    .await
    .map_err(|e| match e {
        AuthError::InvalidCredentials(_) => {
            let error = format!("error={}", Encoded::new(e.to_string()));
            let tag = format!("tag={:x}", {
                let secret: &[u8] = app_state.hmac_secret.expose_secret().as_bytes();
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                mac.update(error.as_bytes());
                mac.finalize().into_bytes()
            });

            LoginError::AuthError(e.into(), Redirect::to(&format!("/login?{error}&{tag}")))
        }
        AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
    })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    Ok(Redirect::to("/"))
}

#[derive(Deserialize)]
pub(super) struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error, Redirect),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self);

        match self {
            Self::AuthError(_, redirect) => redirect.into_response(),
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
