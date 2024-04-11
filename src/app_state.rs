use crate::email_client::EmailClient;
use axum::{extract::FromRef, http::Uri};
use sqlx::PgPool;
use tower_sessions::cookie::Key;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: Uri,
    pub hmac_secret: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.hmac_secret.clone()
    }
}
