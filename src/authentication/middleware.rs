use super::extract::SessionUserId;
use crate::session_state::TypedSession;
use anyhow::anyhow;
use axum::http::{header::LOCATION, HeaderValue, Request, Response, StatusCode};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tower_sessions::Session;
use tracing::Instrument;

#[derive(Clone, Debug)]
pub struct AuthorizedSessionLayer;

impl<S> Layer<S> for AuthorizedSessionLayer {
    type Service = AuthorizedSession<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizedSession { inner }
    }
}

#[derive(Clone, Debug)]
pub struct AuthorizedSession<S> {
    inner: S,
}

impl<S> AuthorizedSession<S> {
    fn see_other<ResBody>() -> Response<ResBody>
    where
        ResBody: Default,
    {
        tracing::info!("User is not logged in");
        let mut res = Response::default();
        *res.status_mut() = StatusCode::SEE_OTHER;
        res.headers_mut()
            .insert(LOCATION, HeaderValue::from_static("/login"));
        res
    }

    fn internal_server_error<ResBody>(error: anyhow::Error) -> Response<ResBody>
    where
        ResBody: Default,
    {
        tracing::error!("{:#?}", error);
        let mut res = Response::default();
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}

impl<ReqBody, ResBody, S> Service<Request<ReqBody>> for AuthorizedSession<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let span = tracing::info_span!("call");
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(
            async move {
                let Some(session) = req
                    .extensions()
                    .get::<Session>()
                    .cloned()
                    .map(TypedSession::new)
                else {
                    return Ok(Self::internal_server_error(anyhow!("Session not found")));
                };

                match session.get_user_id().await {
                    Ok(Some(user_id)) => {
                        tracing::info!("User id `{user_id}` found in session");
                        req.extensions_mut().insert(SessionUserId(user_id));
                    }
                    Ok(None) => return Ok(Self::see_other()),
                    Err(e) => return Ok(Self::internal_server_error(e)),
                };

                inner.call(req).await
            }
            .instrument(span),
        )
    }
}
