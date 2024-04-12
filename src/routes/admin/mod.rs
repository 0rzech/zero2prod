use crate::{app_state::AppState, authentication::middleware::AuthorizedSessionLayer};
use axum::{
    routing::{get, post},
    Router,
};
use dashboard::admin_dashboard;
use logout::log_out;
use password::{change_password, change_password_form};

mod dashboard;
mod logout;
mod password;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest(
            "/admin",
            Router::new()
                .route("/dashboard", get(admin_dashboard))
                .route("/password", get(change_password_form))
                .route("/password", post(change_password))
                .route("/logout", post(log_out)),
        )
        .layer(AuthorizedSessionLayer)
}
