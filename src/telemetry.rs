use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
};
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::{subscriber::set_global_default, Span, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, Registry};
use uuid::Uuid;

pub fn get_subscriber<Sink>(
    name: String,
    default_env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    Registry::default()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_env_filter.into()),
        )
        .with(JsonStorageLayer)
        .with(BunyanFormattingLayer::new(name, sink))
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

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

pub fn request_span(request: &Request<Body>) -> Span {
    let request_id = request
        .headers()
        .get(HeaderName::from_static("x-request-id"))
        .and_then(|value| match value.to_str() {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::warn!("Failed to convert x-request-id to str: {e:?}");
                None
            }
        });

    tracing::info_span!(
        "Request",
        request_id = request_id,
        method = request.method().to_string(),
        path = request.uri().path(),
        query = request.uri().query()
    )
}
