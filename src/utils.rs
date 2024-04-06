use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use std::fmt::Debug;

pub fn redirect_to(uri: &str) -> Response {
    Redirect::to(uri).into_response()
}

pub fn e500<T>(error: T) -> HttpError<T>
where
    T: Debug,
{
    HttpError::InternalServerError(error)
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError<T>
where
    T: Debug,
{
    #[error("Something went wrong")]
    InternalServerError(#[from] T),
}

impl<T> IntoResponse for HttpError<T>
where
    T: Debug,
{
    fn into_response(self) -> Response {
        tracing::error!("{:#?}", self);

        match self {
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
