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
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    SignedCookieJar,
};
use secrecy::Secret;
use serde::Deserialize;

#[tracing::instrument(
    skip(app_state, form, jar),
    fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub(super) async fn login(
    State(app_state): State<AppState>,
    jar: SignedCookieJar,
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
        AuthError::InvalidCredentials(_) => LoginError::AuthErrorWithResponse(e.into(), jar),
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
    AuthErrorWithResponse(#[source] anyhow::Error, SignedCookieJar),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            Self::AuthErrorWithResponse(e, jar) => {
                let error = Self::AuthError(e);
                tracing::error!("{:#?}", error);

                let jar = jar.add(
                    Cookie::build(("_flash", error.to_string()))
                        .http_only(true)
                        .same_site(SameSite::Strict),
                );
                let redirect = Redirect::to("/login");

                (jar, redirect).into_response()
            }
            Self::AuthError(_) => {
                tracing::error!("{:#?}", self);
                StatusCode::UNAUTHORIZED.into_response()
            }
            Self::UnexpectedError(_) => {
                tracing::error!("{:#?}", self);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
