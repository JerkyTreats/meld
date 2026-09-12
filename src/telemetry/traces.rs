//! Optional process tracing. Export and context never authorize semantic work.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

static PROVIDER: OnceLock<Option<SdkTracerProvider>> = OnceLock::new();

pub const ENDPOINT_ENV: &str = "MELD_OTEL_ENDPOINT";
pub const RUN_ENV: &str = "MELD_OTEL_RUN_ID";

pub fn layer() -> Option<impl Layer<Registry> + Send + Sync> {
    let provider = PROVIDER
        .get_or_init(|| {
            let endpoint = std::env::var(ENDPOINT_ENV).ok().filter(|s| !s.is_empty())?;
            let exporter = match opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_protocol(opentelemetry_otlp::Protocol::HttpJson)
                .with_endpoint(endpoint)
                .with_timeout(Duration::from_secs(2))
                .build()
            {
                Ok(exporter) => exporter,
                Err(error) => {
                    eprintln!("Meld trace export unavailable: {error}");
                    return None;
                }
            };
            let executable = std::env::current_exe().ok();
            let service = executable
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "meld".into());
            let service = if std::env::var_os("MELD_OWNER_MAX_MESSAGE_BYTES").is_some() {
                "meld-owner".to_string()
            } else {
                service
            };
            Some(
                SdkTracerProvider::builder()
                    .with_resource(
                        Resource::builder()
                            .with_service_name(service)
                            .with_attributes([
                                KeyValue::new("process.pid", i64::from(std::process::id())),
                                KeyValue::new(
                                    "meld.run_id",
                                    std::env::var(RUN_ENV).unwrap_or_default(),
                                ),
                            ])
                            .build(),
                    )
                    .with_batch_exporter(exporter)
                    .build(),
            )
        })
        .as_ref()?;
    let tracer: SdkTracer = provider.tracer("meld.runtime");
    Some(
        tracing_opentelemetry::layer()
            .with_tracer(tracer)
            .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                metadata.is_span() && metadata.target() == "meld::trace"
            })),
    )
}

/// Package executables share the runtime transport, but own their subscriber.
pub fn init_owner() {
    if let Some(layer) = layer() {
        let _ = Registry::default().with(layer).try_init();
    }
}

pub fn flush() {
    if let Some(Some(provider)) = PROVIDER.get() {
        if let Err(error) = provider.force_flush() {
            eprintln!("Meld trace flush incomplete: {error}");
        }
    }
}

pub fn shutdown() {
    if let Some(Some(provider)) = PROVIDER.get() {
        if let Err(error) = provider.shutdown() {
            eprintln!("Meld trace shutdown incomplete: {error}");
        }
    }
}

/// Standard W3C carrier, omitted when tracing is disabled or unsampled context absent.
pub fn context() -> Option<HashMap<String, String>> {
    let mut carrier = HashMap::new();
    TraceContextPropagator::new().inject_context(&tracing::Span::current().context(), &mut carrier);
    (!carrier.is_empty()).then_some(carrier)
}

pub fn parent(span: &tracing::Span, carrier: Option<&HashMap<String, String>>) {
    if let Some(carrier) = carrier {
        let context = TraceContextPropagator::new().extract(carrier);
        let _ = span.set_parent(context);
    }
}
