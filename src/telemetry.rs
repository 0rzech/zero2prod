use crate::request_id::from_x_request_id;
use axum::{body::Body, http::Request};
use tracing::{subscriber::set_global_default, Span, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, Registry};

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

pub fn request_span(request: &Request<Body>) -> Span {
    tracing::info_span!(
        "Request",
        request_id = from_x_request_id(request),
        method = %request.method(),
        uri = %request.uri(),
    )
}
