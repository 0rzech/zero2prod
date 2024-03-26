use crate::app_state::AppState;
use action::login;
use axum::{
    routing::{get, post},
    Router,
};
use form::login_form;

mod action;
mod form;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_form))
        .route("/login", post(login))
}
