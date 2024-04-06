use crate::app_state::AppState;
use axum::{routing::get, Router};
use dashboard::admin_dashboard;

mod dashboard;

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/dashboard", get(admin_dashboard))
}
