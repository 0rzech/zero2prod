use crate::email_client::EmailClient;
use axum::http::Uri;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: Uri,
}
