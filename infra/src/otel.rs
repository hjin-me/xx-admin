use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer};
use opentelemetry_sdk::Resource;
use std::time::Duration;
use tracing_subscriber::prelude::*;

fn init_tracer(name: &str) -> Tracer {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_timeout(Duration::from_secs(3)),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_max_events_per_span(16)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    name.to_string(),
                )])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap()
}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
pub fn init_tracing_subscriber(name: &str) -> OtelGuard {
    let r = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::try_from_default_env().expect("env filter"))
        .with(tracing_subscriber::fmt::layer());
    if std::env::var(opentelemetry_otlp::OTEL_EXPORTER_OTLP_ENDPOINT).is_ok() {
        r.with(tracing_opentelemetry::layer().with_tracer(init_tracer(name)))
            .init()
    } else {
        r.init()
    }
    OtelGuard {}
}

pub struct OtelGuard {}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
    }
}
