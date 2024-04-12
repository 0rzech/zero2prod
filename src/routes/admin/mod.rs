use crate::{app_state::AppState, authentication::middleware::AuthorizedSessionLayer};
use axum::{
    routing::{get, post},
    Router,
};
use dashboard::admin_dashboard;
use logout::log_out;
use newsletters::{newsletter_form, publish_newsletter};
use password::{change_password, change_password_form};

mod dashboard;
mod logout;
mod newsletters;
mod password;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest(
            "/admin",
            Router::new()
                .route("/dashboard", get(admin_dashboard))
                .route("/newsletters", get(newsletter_form))
                .route("/newsletters", post(publish_newsletter))
                .route("/password", get(change_password_form))
                .route("/password", post(change_password))
                .route("/logout", post(log_out)),
        )
        .layer(AuthorizedSessionLayer)
}
