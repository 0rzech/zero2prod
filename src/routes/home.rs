use crate::app_state::AppState;
use askama_axum::Template;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(home))
}

#[tracing::instrument(name = "Render home page")]
async fn home() -> HomeTemplate<'static> {
    HomeTemplate {
        title: "zero2prod",
        username: None,
    }
}

#[derive(Template)]
#[template(path = "web/home.html")]
struct HomeTemplate<'a> {
    title: &'a str,
    username: Option<String>,
}
