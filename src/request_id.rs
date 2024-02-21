use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
};
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestUuid;

impl MakeRequestId for RequestUuid {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        match HeaderValue::from_str(&Uuid::new_v4().to_string()) {
            Ok(value) => Some(RequestId::new(value)),
            Err(e) => {
                tracing::warn!("Failed to create request id header value: {e:?}");
                None
            }
        }
    }
}

pub fn from_x_request_id(request: &Request<Body>) -> Option<&str> {
    request
        .headers()
        .get(HeaderName::from_static("x-request-id"))
        .and_then(|value| match value.to_str() {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::warn!("Failed to convert x-request-id to str: {e:?}");
                None
            }
        })
}
