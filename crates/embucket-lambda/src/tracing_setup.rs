use api_snowflake_rest::server::error;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use std::collections::HashMap;
use tracing::log::LevelFilter;
use tracing::Subscriber;
use tracing::subscriber::set_global_default;
use tracing_subscriber::filter::{EnvFilter, FilterExt, filter_fn};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

/// Configures and initializes tracing, returning a provider for graceful shutdown
///
/// This function is an adaptation of the binary's `setup_tracing` for a Lambda environment
/// It uses environment variables for configuration instead of CLI options
pub fn init_tracing() -> SdkTracerProvider {
    let otlp_endpoint = std::env::var("OPENTELEMETRY_ENDPOINT_URL")
        .unwrap_or_else(|_| "https://api.honeycomb.io/".to_string());

    let api_key = std::env::var("HONEYCOMB_API_KEY").expect("HONEYCOMB_API_KEY must be set");
    let dataset = std::env::var("HONEYCOMB_DATASET").expect("HONEYCOMB_DATASET must be set");

    let mut headers = HashMap::with_capacity(2);
    headers.insert("x-honeycomb-dataset".to_string(), dataset.parse().unwrap());
    headers.insert("x-honeycomb-team".to_string(), api_key.parse().unwrap());

    let exporter = opentelemetry_otlp::HttpExporterBuilder::default()
        .with_endpoint(otlp_endpoint)
        .with_http_client(reqwest::Client::default())
        .with_headers(headers)
        .with_timeout(std::time::Duration::from_secs(3))
        .build_span_exporter()
        .expect("Failed to create OTLP SpanExporter");

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "embucket-lambda-api".to_string());

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(Resource::builder().with_service_name(service_name.clone()).build())
        .build();

    let tracer = tracer_provider.tracer(service_name); // The name of the tracer
    global::set_tracer_provider(tracer_provider.clone());

    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_filter = filter_fn(|meta| meta.is_event() || meta.is_span());

    Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer()
            .json()
            .with_target(false)
            .with_ansi(false)
            .with_current_span(false)
            .with_span_list(false)
            .without_time()
        )
        .with(fmt_filter)
        .with(otel_layer)
        .init();

    global::set_text_map_propagator(TraceContextPropagator::new());

    tracer_provider
}

#[lambda_http::tracing::instrument]
pub async fn trace_flusher(
    State(state): State<SdkTracerProvider>,
    req: Request,
    next: Next,
) -> error::Result<impl IntoResponse> {
    let response = next.run(req).await;

    let _ = state.force_flush();

    Ok(response)
}
