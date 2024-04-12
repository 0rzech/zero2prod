use crate::{
    app_state::AppState,
    authentication::password::{validate_credentials, AuthError, Credentials},
    session_state::TypedSession,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_messages::Messages;
use secrecy::Secret;
use serde::Deserialize;

#[tracing::instrument(
    skip(app_state, session, messages, form),
    fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub(super) async fn login(
    State(app_state): State<AppState>,
    session: TypedSession,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Redirect, LoginErrorResponse> {
    tracing::Span::current().record("username", &tracing::field::display(&form.username));

    let user_id = match validate_credentials(
        &app_state.db_pool,
        Credentials {
            username: form.username,
            password: form.password,
        },
    )
    .await
    {
        Ok(user_id) => user_id,
        Err(e) => match e {
            AuthError::InvalidCredentials(_) => {
                return Err(LoginErrorResponse::new_auth_with_redirect(
                    e.into(),
                    messages,
                ));
            }
            AuthError::UnexpectedError(_) => {
                return Err(LoginErrorResponse::new_unexpected(e.into()));
            }
        },
    };

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    if let Err(e) = session.cycle_id().await {
        return Err(LoginErrorResponse::new_unexpected_with_redirect(
            e, messages,
        ));
    }

    session
        .insert_user_id(user_id)
        .await
        .map_err(|e| LoginErrorResponse::new_unexpected_with_redirect(e, messages))?;

    Ok(Redirect::to("/admin/dashboard"))
}

#[derive(Deserialize)]
pub(super) struct FormData {
    username: String,
    password: Secret<String>,
}

pub(super) struct LoginErrorResponse {
    error: LoginError,
    messages: Option<Messages>,
}

impl LoginErrorResponse {
    fn new_unexpected(error: anyhow::Error) -> Self {
        Self {
            error: LoginError::UnexpectedError(error),
            messages: None,
        }
    }

    fn new_auth_with_redirect(error: anyhow::Error, messages: Messages) -> Self {
        Self {
            error: LoginError::AuthError(error),
            messages: Some(messages),
        }
    }

    fn new_unexpected_with_redirect(error: anyhow::Error, messages: Messages) -> Self {
        Self {
            error: LoginError::UnexpectedError(error),
            messages: Some(messages),
        }
    }
}

impl IntoResponse for LoginErrorResponse {
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self.error);

        match (self.error, self.messages) {
            (error, Some(messages)) => {
                messages.error(error.to_string());
                Redirect::to("/login").into_response()
            }
            (LoginError::AuthError(_), None) => StatusCode::UNAUTHORIZED.into_response(),
            (LoginError::UnexpectedError(_), None) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}
