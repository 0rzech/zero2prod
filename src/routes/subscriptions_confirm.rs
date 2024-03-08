use crate::app_state::AppState;
use axum::{extract::Query, http::StatusCode, routing::get, Router};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions/confirm", get(confirm))
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
async fn confirm(Query(_parameters): Query<Parameters>) -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
struct Parameters {
    #[allow(dead_code)]
    subscription_token: String,
}
