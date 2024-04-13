use crate::{
    app_state::AppState,
    authentication::{
        extract::SessionUserId,
        password::{
            change_password as auth_change_password, validate_credentials, AuthError, Credentials,
        },
    },
    routes::admin::dashboard::get_username,
    utils::{e500, HttpError},
};
use axum::{extract::State, response::Redirect, Form};
use axum_messages::Messages;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[tracing::instrument(skip(app_state, user_id, messages, form))]
pub(in crate::routes::admin) async fn change_password(
    State(app_state): State<AppState>,
    SessionUserId(user_id): SessionUserId,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Redirect, HttpError<anyhow::Error>> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        messages.error(
            "You have entered two different new passwords - \
            the field values must match.",
        );
        return Ok(Redirect::to("/admin/password"));
    }

    let credentials = get_username(&app_state.db_pool, user_id)
        .await
        .map(|username| Credentials {
            username,
            password: form.current_password,
        })
        .map_err(e500)?;

    if let Err(e) = validate_credentials(&app_state.db_pool, credentials).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                messages.error("The current password is incorrect.");
                Ok(Redirect::to("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e.into())),
        };
    }

    if let Err(e) = validate_password(&form.new_password) {
        messages.error(e);
        return Ok(Redirect::to("/admin/password"));
    }

    auth_change_password(&app_state.db_pool, user_id, form.new_password)
        .await
        .map_err(e500)?;

    messages.info("Your password has been changed.");

    Ok(Redirect::to("/admin/password"))
}

#[derive(Deserialize)]
pub(in crate::routes::admin) struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

fn validate_password(password: &Secret<String>) -> Result<(), &'static str> {
    let password = password.expose_secret();

    if password.len() < 12 {
        return Err("Password must be at least 12 characters long.");
    }

    if password.len() > 128 {
        return Err("Passwords must be at most 128 characters long.");
    }

    Ok(())
}
