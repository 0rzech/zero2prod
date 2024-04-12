use crate::app_state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use get::login_form;
use post::login;

mod get;
mod post;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_form))
        .route("/login", post(login))
}
