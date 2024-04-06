use crate::{
    app_state::AppState,
    session_state::TypedSession,
    utils::{e500, redirect_to, HttpError},
};
use anyhow::{Context, Error};
use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, response::Response};
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get admin dashboard", skip(app_state, session))]
pub(super) async fn admin_dashboard(
    State(app_state): State<AppState>,
    session: TypedSession,
) -> Result<Response, HttpError<Error>> {
    let response = match session.get_user_id().await.map_err(e500)? {
        Some(user_id) => Dashboard {
            title: "Admin Dashboard",
            username: get_username(&app_state.db_pool, user_id)
                .await
                .map_err(e500)?,
        }
        .into_response(),
        None => redirect_to("/login"),
    };

    Ok(response)
}

#[tracing::instrument(skip(db_pool))]
async fn get_username(db_pool: &PgPool, user_id: Uuid) -> Result<String, Error> {
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
struct Dashboard<'a> {
    title: &'a str,
    username: String,
}
