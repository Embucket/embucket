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
use tracing::Subscriber;
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
    global::set_text_map_propagator(TraceContextPropagator::new());

    let otlp_endpoint = std::env::var("OPENTELEMETRY_ENDPOINT_URL")
        .unwrap_or_else(|_| "https://api.honeycomb.io/".to_string());

    // --> START: Honeycomb-specific configuration
    let api_key = std::env::var("HONEYCOMB_API_KEY").expect("HONEYCOMB_API_KEY must be set");
    let dataset = std::env::var("HONEYCOMB_DATASET").expect("HONEYCOMB_DATASET must be set");

    let mut headers = HashMap::with_capacity(2);
    headers.insert("x-honeycomb-team".to_string(), api_key.parse().unwrap());
    headers.insert("x-honeycomb-dataset".to_string(), dataset.parse().unwrap());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(otlp_endpoint)
        .with_http_client(reqwest::Client::new())
        .with_headers(headers)
        .with_timeout(std::time::Duration::from_secs(3))
        .build()
        .expect("Failed to create OTLP SpanExporter");

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(resource())
        .build();

    let tracer = tracer_provider.tracer("embucket-lambda"); // The name of the tracer
    global::set_tracer_provider(tracer_provider.clone());

    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(EnvFilter::from_default_env());

    let fmt_layer = build_fmt_layer();

    Registry::default().with(otel_layer).with(fmt_layer).init();

    tracer_provider
}

/// Creates a Resource that identifies this service in observability tools
fn resource() -> Resource {
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "embucket-lambda-api".to_string());
    Resource::builder().with_service_name(service_name).build()
}

/// Builds the log formatting layer, preserving the json/pretty logic
fn build_fmt_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    // RUST_LOG
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let span_events_filter = filter_fn(|meta| meta.is_event() || meta.is_span());

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    if log_format == "json" {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(false)
            .with_ansi(false)
            .with_current_span(false)
            .with_span_list(false)
            .without_time()
            // Apply the filter only to log events, not span open/close.
            .with_filter(span_events_filter.and(fmt_filter))
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_filter(span_events_filter.and(fmt_filter))
            .boxed()
    }
}
#[lambda_http::tracing::instrument]
pub async fn trace_flusher(
    State(state): State<SdkTracerProvider>,
    req: Request,
    next: Next,
) -> error::Result<impl IntoResponse> {
    let response = next.run(req).await;

    let flush_result = state.force_flush();

    tracing::Span::current().record("flush_result", format!("{flush_result:#?}"));

    Ok(response)
}
