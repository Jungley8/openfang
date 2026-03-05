//! Tracing initialization with optional OpenTelemetry OTLP export.
//!
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is set, registers an OTLP trace exporter
//! so spans are sent to Jaeger, Honeycomb, or other OTLP backends.

use opentelemetry::trace::TracerProvider;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Set W3C TraceContext as the global propagator so API can extract from headers.
fn set_propagator() {
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    opentelemetry::global::set_text_map_propagator(propagator);
}

/// Build the OTLP span exporter and tracer when endpoint is configured.
/// Returns a tracer suitable for OpenTelemetryLayer, or None if disabled/failed.
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
        Some(provider.tracer("octarq"))
    })
}

/// Initialize tracing to stderr with optional OTLP export.
pub fn init_tracing_stderr() {
    set_propagator();
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
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

/// Initialize tracing to a log file (for TUI modes) with optional OTLP export.
pub fn init_tracing_file() {
    set_propagator();
    let log_dir = dirs::home_dir()
        .map(|h| h.join(".openfang"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("tui.log");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_result = std::fs::File::create(&log_path);
    let tracer = try_otel_tracer();

    match (file_result, tracer) {
        (Ok(file), Some(tracer)) => tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false),
            )
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init(),
        (Ok(file), None) => tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false),
            )
            .init(),
        (Err(_), Some(tracer)) => tracing_subscriber::registry()
            .with(EnvFilter::new("error"))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::sink)
                    .with_ansi(false),
            )
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init(),
        (Err(_), None) => tracing_subscriber::registry()
            .with(EnvFilter::new("error"))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::sink)
                    .with_ansi(false),
            )
            .init(),
    }
}
