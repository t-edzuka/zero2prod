use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn get_subscriber<Sink>(
    app_name: impl Into<String>,
    log_level: impl Into<String>,
    sink: Sink,
) -> impl Subscriber + Send + Sync
// Sink trait bound, which is very weird, is Higher-Ranked Trait Bound (HRTB)
// See https://doc.rust-lang.org/nomicon/hrtb.html in detail.
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level.into()));
    let formatting_layer = BunyanFormattingLayer::new(app_name.into(), sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Redirect all `log`'s events to out `subscriber` defined below.
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
