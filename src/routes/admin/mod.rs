use crate::app_state::AppState;
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
        .route("/admin/dashboard", get(admin_dashboard))
        .route("/admin/password", get(change_password_form))
        .route("/admin/password", post(change_password))
        .route("/admin/logout", post(log_out))
}
