use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SessionUserId(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for SessionUserId
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<SessionUserId>().cloned().ok_or({
            tracing::error!("User id not found in session");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
}
