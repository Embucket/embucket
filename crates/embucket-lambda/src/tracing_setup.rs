use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_otlp::WithHttpConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use std::collections::HashMap;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

/// Configure OpenTelemetry OTLP exporter to Honeycomb and return the tracer provider.
/// In Lambda, this should be called once during init; remember to force_flush after each invocation.
pub fn init_tracing() -> SdkTracerProvider {
    // Service name visible in Honeycomb
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "embucket-lambda".to_string());

    // Select endpoint and protocol from env (defaults: HTTP, US endpoint)
    let endpoint = std::env::var("HONEYCOMB_API_ENDPOINT")
        .ok()
        .or_else(|| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok())
        .unwrap_or_else(|| "https://api.honeycomb.io".to_string());
    let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
        .unwrap_or_else(|_| "http/protobuf".to_string());

    // Allow hard disabling via env (useful for prod hotfixes):
    let sdk_disabled = std::env::var("OTEL_SDK_DISABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    // Decide whether we have credentials; if not, skip exporter entirely.
    let hc_api_key = std::env::var("HONEYCOMB_API_KEY").ok();
    let hc_dataset = std::env::var("HONEYCOMB_DATASET").ok();

    // Build OTLP exporter (HTTP by default; gRPC if explicitly requested). Any failure should NOT
    // abort the Lambda. We log and proceed without an exporter so invocations still work.
    let span_exporter = if sdk_disabled {
        tracing::warn!("OTEL_SDK_DISABLED is set; OpenTelemetry exporter disabled");
        None
    } else { match (hc_api_key.as_deref(), hc_dataset.as_deref()) {
        (Some(api_key), Some(dataset)) => {
            if protocol.eq_ignore_ascii_case("grpc") {
                // gRPC to Honeycomb
                let mut builder = opentelemetry_otlp::SpanExporterBuilder::default()
                    .with_tonic()
                    .with_endpoint(&endpoint);

                let mut md = MetadataMap::with_capacity(2);
                if let Ok(v) = api_key.parse() { md.insert("x-honeycomb-team", v); }
                if let Ok(v) = dataset.parse() { md.insert("x-honeycomb-dataset", v); }
                builder = builder.with_metadata(md);

                match builder
                    .with_timeout(std::time::Duration::from_secs(10))
                    .build()
                {
                    Ok(exp) => {
                        tracing::info!(endpoint=%endpoint, dataset=%dataset, protocol="grpc", "Configured Honeycomb OTLP exporter");
                        Some(exp)
                    },
                    Err(err) => {
                        tracing::error!(%err, endpoint=%endpoint, protocol=%protocol, "Failed to create OTLP gRPC span exporter for Honeycomb; continuing without exporter");
                        None
                    }
                }
            } else {
                // HTTP/protobuf to Honeycomb (default)
                // Ensure we send to the OTLP HTTP traces path.
                let http_endpoint = if endpoint.contains("/v1/traces") {
                    endpoint.clone()
                } else if endpoint.ends_with('/') {
                    format!("{}v1/traces", endpoint)
                } else {
                    format!("{}/v1/traces", endpoint)
                };

                let mut builder = opentelemetry_otlp::SpanExporterBuilder::default()
                    .with_http()
                    .with_endpoint(&http_endpoint);

                let mut headers = HashMap::with_capacity(2);
                headers.insert("x-honeycomb-team".to_string(), api_key.to_string());
                headers.insert("x-honeycomb-dataset".to_string(), dataset.to_string());
                builder = builder.with_headers(headers);

                match builder
                    .with_timeout(std::time::Duration::from_secs(10))
                    .build()
                {
                    Ok(exp) => {
                        tracing::info!(endpoint=%http_endpoint, dataset=%dataset, protocol="http/protobuf", "Configured Honeycomb OTLP exporter");
                        Some(exp)
                    },
                    Err(err) => {
                        tracing::error!(%err, endpoint=%http_endpoint, protocol=%protocol, "Failed to create OTLP HTTP span exporter for Honeycomb; continuing without exporter");
                        None
                    }
                }
            }
        }
        _ => {
            tracing::info!("HONEYCOMB_API_KEY or HONEYCOMB_DATASET not set; OpenTelemetry exporter disabled");
            None
        }
    }};

    let mut provider_builder = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name(service_name.clone()).build());

    if let Some(exporter) = span_exporter {
        provider_builder = provider_builder
            .with_span_processor(BatchSpanProcessor::builder(exporter).build());
    } else {
        tracing::warn!("OpenTelemetry exporter not configured; traces will not be sent");
    }

    let provider = provider_builder.build();

    let tracer = provider.tracer(service_name);
    global::set_tracer_provider(provider.clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .with(otel_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting global tracing subscriber failed");

    // Use W3C TraceContext propagation
    global::set_text_map_propagator(TraceContextPropagator::new());

    provider
}

#[lambda_http::tracing::instrument]
pub async fn trace_flusher(
    State(state): State<SdkTracerProvider>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let response = next.run(req).await;

    let _ = state.force_flush();

    response
}
