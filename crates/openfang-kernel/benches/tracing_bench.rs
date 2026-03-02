//! Benchmark: send_message path (POST /api/agents/{id}/message → execute_llm_agent → run_agent_loop).
//!
//! Run with mock LLM to control variables. Compare three tracing modes:
//!
//! - `BENCH_OTEL=none`   — fmt only (no OpenTelemetry)
//! - `BENCH_OTEL=memory` — OTel with noop exporter (span creation, no network)
//! - `BENCH_OTEL=otlp`   — OTel + OTLP gRPC (set OTEL_EXPORTER_OTLP_ENDPOINT)
//!
//! Example:
//!   BENCH_OTEL=none   cargo bench -p openfang-kernel --bench tracing_bench --features bench
//!   BENCH_OTEL=memory cargo bench -p openfang-kernel --bench tracing_bench --features bench
//!   OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 BENCH_OTEL=otlp cargo bench ...

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use openfang_kernel::OpenFangKernel;
use openfang_types::agent::AgentManifest;
use openfang_types::config::{DefaultModelConfig, KernelConfig};
use opentelemetry::trace::TracerProvider;
use std::sync::Arc;
use std::sync::Once;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static TRACING_INIT: Once = Once::new();

fn init_tracing_for_bench() {
    TRACING_INIT.call_once(|| {
        let mode = std::env::var("BENCH_OTEL").unwrap_or_else(|_| "none".into());
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error"));
        let fmt = tracing_subscriber::fmt::layer().with_writer(std::io::sink);

        match mode.as_str() {
            "none" => {
                tracing_subscriber::registry().with(filter).with(fmt).init();
            }
            "memory" => {
                let noop = NoopSpanExporter;
                let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                    .with_batch_exporter(noop)
                    .build();
                opentelemetry::global::set_tracer_provider(provider.clone());
                let tracer = provider.tracer("openfang");
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt)
                    .with(tracing_opentelemetry::layer().with_tracer(tracer))
                    .init();
            }
            "otlp" => {
                if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let tracer = rt.block_on(async {
                        let exporter = opentelemetry_otlp::SpanExporter::builder()
                            .with_tonic()
                            .build()
                            .ok()?;
                        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                            .with_batch_exporter(exporter)
                            .build();
                        opentelemetry::global::set_tracer_provider(provider.clone());
                        Some(provider.tracer("openfang"))
                    });
                    if let Some(tracer) = tracer {
                        tracing_subscriber::registry()
                            .with(filter)
                            .with(fmt)
                            .with(tracing_opentelemetry::layer().with_tracer(tracer))
                            .init();
                    } else {
                        tracing_subscriber::registry().with(filter).with(fmt).init();
                    }
                } else {
                    tracing_subscriber::registry().with(filter).with(fmt).init();
                }
            }
            _ => {
                tracing_subscriber::registry().with(filter).with(fmt).init();
            }
        }
    });
}

/// Noop span exporter for "OTel memory" mode (spans created and processed, not sent).
#[derive(Debug)]
struct NoopSpanExporter;

#[allow(refining_impl_trait)]
impl opentelemetry_sdk::trace::SpanExporter for NoopSpanExporter {
    fn export(
        &self,
        _batch: Vec<opentelemetry_sdk::trace::SpanData>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), opentelemetry_sdk::error::OTelSdkError>>
                + Send,
        >,
    > {
        Box::pin(async { Ok(()) })
    }
}

const MOCK_MANIFEST: &str = r#"
name = "bench-agent"
version = "0.1.0"
description = "Benchmark agent"
module = "builtin:chat"

[model]
provider = "mock"
model = "mock"
system_prompt = "You are a test agent."

[capabilities]
tools = []
memory_read = []
memory_write = []
"#;

async fn setup_kernel_and_agent() -> (Arc<OpenFangKernel>, openfang_types::agent::AgentId) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        default_model: DefaultModelConfig {
            provider: "mock".to_string(),
            model: "mock".to_string(),
            api_key_env: "MOCK_API_KEY".to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    };

    let kernel = OpenFangKernel::boot_with_config(config).expect("kernel boot");
    let kernel = Arc::new(kernel);
    kernel.set_self_handle();

    let manifest: AgentManifest = toml::from_str(MOCK_MANIFEST).expect("manifest parse");
    let agent_id = kernel.spawn_agent(manifest).expect("spawn agent");

    (kernel, agent_id)
}

fn bench_send_message(c: &mut Criterion) {
    init_tracing_for_bench();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let (kernel, agent_id) = rt.block_on(setup_kernel_and_agent());

    let mode = std::env::var("BENCH_OTEL").unwrap_or_else(|_| "none".into());
    let group_name = format!("send_message_{}", mode);

    c.bench_function(&group_name, |b| {
        b.iter(|| {
            let k = Arc::clone(&kernel);
            let id = agent_id;
            rt.block_on(async move {
                let _ = k.send_message(id, black_box("Hi")).await;
            });
        });
    });
}

criterion_group!(benches, bench_send_message);
criterion_main!(benches);
