use api_snowflake_rest::server::error;
use api_snowflake_rest_sessions::layer::Host;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig, WithTonicConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use tracing::{Subscriber, instrument};
use tracing_subscriber::filter::{EnvFilter, FilterExt, filter_fn};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

/// Configures and initializes tracing, returning a provider for graceful shutdown
///
/// This function is an adaptation of the binary's `setup_tracing` for a Lambda environment
/// It uses environment variables for configuration instead of CLI options
pub fn init_tracing() -> SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
        .unwrap_or_else(|_| "http/protobuf".to_string());

    let exporter = if protocol == "grpc" {
        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string());

        opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
            .expect("Failed to create OTLP HTTP SpanExporter")
    } else {
        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());

        opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .expect("Failed to create OTLP HTTP SpanExporter")
    };

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(resource())
        .build();

    let tracer = tracer_provider.tracer("embucket-lambda");
    global::set_tracer_provider(tracer_provider.clone());

    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(EnvFilter::from_default_env());

    Registry::default()
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer().json().with_ansi(false))
        .init();

    tracer_provider
}

/// Creates a Resource that identifies this service in observability tools
fn resource() -> Resource {
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "embucketd-lambda-api".to_string());
    Resource::builder().with_service_name(service_name).build()
}

/// Builds the log formatting layer, preserving the json/pretty logic
fn build_fmt_layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    // RUST_LOG
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let span_events_filter = filter_fn(|meta| meta.is_span() || meta.is_event());

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    if log_format == "json" {
        tracing_subscriber::fmt::layer()
            .json()
            // this needs to be set to remove duplicated information in the log.
            .with_current_span(false)
            // this needs to be set to false, otherwise ANSI color codes will
            // show up in a confusing manner in CloudWatch logs.
            .with_ansi(false)
            // disabling time is handy because CloudWatch will add the ingestion time.
            .without_time()
            // remove the name of the function from every log entry
            .with_target(false)
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

    let _ = state.force_flush();

    Ok(response)
}
