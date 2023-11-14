use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer};
use opentelemetry_sdk::Resource;
use std::time::Duration;
use tracing::Level;
use tracing_subscriber::prelude::*;

fn init_tracer(name: &str, endpoint: &str) -> Tracer {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
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
        .unwrap();
    tracer
}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
pub fn init_tracing_subscriber(name: &str, endpoint: &str) -> OtelGuard {
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(init_tracer(name, endpoint)))
        .init();
    OtelGuard {}
}

pub struct OtelGuard {}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use std::{thread, time::Duration};
    use tracing::info;
    use tracing::{instrument, span, trace, warn};
    #[instrument]
    #[inline]
    fn expensive_work() -> &'static str {
        span!(tracing::Level::INFO, "expensive_step_1")
            .in_scope(|| thread::sleep(Duration::from_millis(25)));
        span!(tracing::Level::INFO, "expensive_step_2")
            .in_scope(|| thread::sleep(Duration::from_millis(25)));

        "success"
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_trace() -> Result<()> {
        // let _ = init_jaeger();
        let _g = init_tracing_subscriber("", "");
        info!("CARGO_PKG_NAME={}", env!("CARGO_PKG_NAME"));

        {
            let root = span!(tracing::Level::INFO, "app_start_otel_env", work_units = 2);
            let _enter = root.enter();

            let work_result = expensive_work();

            span!(tracing::Level::INFO, "faster_work")
                .in_scope(|| thread::sleep(Duration::from_millis(10)));

            warn!("About to exit!");
            trace!("status: {}", work_result);
        } // Once this scope is closed, all spans inside are closed as well
        Ok(())
    }
}
