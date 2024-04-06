use anyhow::{Context, Error};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use tower_sessions::Session;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub async fn cycle_id(&self) -> Result<(), Error> {
        self.0
            .cycle_id()
            .await
            .context("Failed to cycle session id")
    }

    pub async fn insert_user_id(&self, user_id: Uuid) -> Result<(), Error> {
        self.0
            .insert(Self::USER_ID_KEY, user_id)
            .await
            .context("Failed to insert user id into session")
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>, Error> {
        self.0
            .get(Self::USER_ID_KEY)
            .await
            .context("Failed to retrieve user id from session")
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(req, state).await?;
        Ok(TypedSession(session))
    }
}
