//! Tracing initialization with optional OpenTelemetry OTLP export.

use opentelemetry::trace::TracerProvider;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn set_propagator() {
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    opentelemetry::global::set_text_map_propagator(propagator);
}

fn try_otel_tracer() -> Option<opentelemetry_sdk::trace::Tracer> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    let rt = tokio::runtime::Runtime::new().ok()?;
    rt.block_on(async {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .build()
            .ok()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        opentelemetry::global::set_tracer_provider(provider.clone());
        Some(provider.tracer("openfang"))
    })
}

pub fn init_tracing() {
    set_propagator();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("openfang=info,tauri=info"));
    let fmt_layer = tracing_subscriber::fmt::layer();
    match try_otel_tracer() {
        Some(tracer) => tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init(),
        None => tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init(),
    }
}
