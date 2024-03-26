use crate::app_state::AppState;
use anyhow::Context;
use askama_axum::Template;
use axum::extract::{Query, State};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use urlencoding::Encoded;

#[tracing::instrument(skip(app_state, parameters))]
pub(super) async fn login_form(
    State(app_state): State<AppState>,
    Query(parameters): Query<Parameters>,
) -> LoginForm<'static> {
    let error_message = match parameters.error_message(&app_state.hmac_secret) {
        Ok(raw_html) => raw_html,
        Err(e) => {
            tracing::warn!("Failed to get error message from query parameters: {:?}", e);
            None
        }
    }
    .map(|raw_html| htmlescape::encode_minimal(&raw_html));

    LoginForm {
        title: "Login",
        username_label: "Username",
        username_placeholder: "Enter username",
        password_label: "Password",
        password_placeholder: "Enter password",
        submit_label: "Login",
        error_message,
        action: "/login",
    }
}

#[derive(Deserialize)]
pub(super) struct Parameters {
    error: Option<String>,
    tag: Option<String>,
}

impl Parameters {
    fn error_message(self, hmac_secret: &Secret<String>) -> Result<Option<String>, anyhow::Error> {
        match (&self.error, self.tag) {
            (Some(e), Some(t)) => {
                let tag = hex::decode(t).context("Failed to decode hex hmac tag")?;
                let error = format!("error={}", Encoded::new(e));

                let mut mac =
                    Hmac::<sha2::Sha256>::new_from_slice(hmac_secret.expose_secret().as_bytes())?;
                mac.update(error.as_bytes());
                mac.verify_slice(&tag)?;

                Ok(self.error)
            }
            (None, None) => Ok(None),
            (Some(_), None) => Err(anyhow::anyhow!("Error message is missing hmac tag")),
            (None, Some(_)) => Err(anyhow::anyhow!("Hmac tag is missing error message")),
        }
    }
}

#[derive(Template)]
#[template(path = "web/login_form.html")]
pub(super) struct LoginForm<'a> {
    title: &'a str,
    username_label: &'a str,
    username_placeholder: &'a str,
    password_label: &'a str,
    password_placeholder: &'a str,
    submit_label: &'a str,
    error_message: Option<String>,
    action: &'a str,
}
