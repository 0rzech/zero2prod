use crate::{
    app_state::AppState,
    session::extract::SessionUserId,
    utils::{e500, HttpError},
};
use anyhow::{Context, Error};
use askama::Template;
use axum::extract::State;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get admin dashboard", skip(app_state, user_id))]
pub(super) async fn admin_dashboard(
    State(app_state): State<AppState>,
    SessionUserId(user_id): SessionUserId,
) -> Result<Dashboard<'static>, HttpError<Error>> {
    let username = get_username(&app_state.db_pool, user_id)
        .await
        .map_err(e500)?;

    Ok(Dashboard {
        title: "Admin Dashboard",
        welcome: "Welcome",
        available_actions: "Available actions",
        change_password: "Change password",
        logout: "Logout",
        username,
    })
}

#[tracing::instrument(skip(db_pool, user_id))]
pub async fn get_username(db_pool: &PgPool, user_id: Uuid) -> Result<String, Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;

    Ok(row.username)
}

#[derive(Template)]
#[template(path = "web/dashboard.html")]
pub struct Dashboard<'a> {
    title: &'a str,
    welcome: &'a str,
    available_actions: &'a str,
    change_password: &'a str,
    logout: &'a str,
    username: String,
}
