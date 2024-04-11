use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

pub struct SessionUserId(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for SessionUserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Uuid>()
            .cloned()
            .map(SessionUserId)
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "User id not present in session",
            ))
    }
}
