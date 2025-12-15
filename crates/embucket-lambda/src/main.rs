mod config;

use crate::config::EnvConfig;
use api_snowflake_rest::server::core_state::CoreState;
use api_snowflake_rest::server::core_state::MetastoreConfig;
use api_snowflake_rest::server::make_snowflake_router;
use api_snowflake_rest::server::server_models::RestApiConfig as SnowflakeServerConfig;
use api_snowflake_rest::server::state::AppState;
use api_snowflake_rest_sessions::session::SESSION_EXPIRATION_SECONDS;
use axum::{middleware, Router};
use axum::body::Body as AxumBody;
use axum::extract::connect_info::ConnectInfo;
use catalog_metastore::metastore_settings_config::MetastoreSettingsConfig;
use http::HeaderMap;
use http_body_util::BodyExt;
use lambda_http::{Body as LambdaBody, Error as LambdaError, Request, Response, service_fn};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::io::IsTerminal;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use opentelemetry_otlp::WithExportConfig;
use tower::ServiceExt;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use axum::extract::{Request as AxumRequest, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
cfg_if::cfg_if! {
    if #[cfg(feature = "streaming")] {
        use lambda_http::run_with_streaming_response as run;
    } else {
        use lambda_http::run;
    }
}

type InitResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const DISABLED_TARGETS: [&str; 2] = ["h2", "aws_smithy_runtime"];

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    let env_config = EnvConfig::from_env();

    let tracing_provider = init_tracing_and_logs(&env_config);

    info!(
        data_format = %env_config.data_format,
        max_concurrency = env_config.max_concurrency_level,
        query_timeout_secs = env_config.query_timeout_secs,
        mem_pool_type = ?env_config.mem_pool_type,
        mem_pool_size_mb = ?env_config.mem_pool_size_mb,
        disk_pool_size_mb = ?env_config.disk_pool_size_mb,
        aws_sdk_connect_timeout_secs = env_config.aws_sdk_connect_timeout_secs,
        aws_sdk_operation_timeout_secs = env_config.aws_sdk_operation_timeout_secs,
        aws_sdk_operation_attempt_timeout_secs = env_config.aws_sdk_operation_attempt_timeout_secs,
        metastore_config = env_config.metastore_config.as_ref().map(|p| p.display().to_string()),
        object_store_timeout_secs = env_config.object_store_timeout_secs,
        object_store_connect_timeout_secs = env_config.object_store_connect_timeout_secs,
        "Loaded Lambda configuration"
    );

    let app = Arc::new(LambdaApp::initialize(env_config).await.map_err(|err| {
        error!(error = %err, "Failed to initialize Lambda services");
        err
    })?);

    let err = run(service_fn(move |event: Request| {
        let app = Arc::clone(&app);
        async move { app.handle_event(event).await }
    }))
    .await;

    tracing_provider.shutdown().map_err(|err| {
        error!(error = %err, "Failed to shutdown TracerProvider");
        err
    })?;

    err
}

struct LambdaApp {
    router: Router,
}

impl LambdaApp {
    #[tracing::instrument(name = "lambda_app_initialize", skip_all, fields(
        data_format = %config.data_format,
        max_concurrency = config.max_concurrency_level
    ))]
    async fn initialize(config: EnvConfig) -> InitResult<Self> {
        let snowflake_cfg = SnowflakeServerConfig::new(
            &config.data_format,
            config.jwt_secret.clone().unwrap_or_default(),
        )?
        .with_demo_credentials(
            config.auth_demo_user.clone(),
            config.auth_demo_password.clone(),
        );
        let execution_cfg = config.execution_config();

        let metastore_cfg = if let Some(config_path) = &config.metastore_config {
            MetastoreConfig::ConfigPath(config_path.clone())
        } else {
            MetastoreConfig::None
        };

        let metastore_settings_config = MetastoreSettingsConfig::default()
            .with_object_store_timeout(config.object_store_timeout_secs)
            .with_object_store_connect_timeout(config.object_store_connect_timeout_secs);

        let core_state = CoreState::new(
            execution_cfg,
            snowflake_cfg,
            metastore_settings_config,
            metastore_cfg,
        )
        .await?;
        core_state
            .with_session_timeout(tokio::time::Duration::from_secs(SESSION_EXPIRATION_SECONDS))?;

        let appstate = AppState::from(&core_state);
        let router = make_snowflake_router(appstate);
        info!("Initialized Lambda Snowflake REST services");

        Ok(Self { router })
    }

    #[tracing::instrument(name = "lambda_handle_event", skip_all, fields(
        http.method = %request.method(),
        http.uri = %request.uri(),
        http.request_id = tracing::field::Empty,
        http.status_code = tracing::field::Empty
    ))]
    async fn handle_event(&self, request: Request) -> Result<Response<LambdaBody>, LambdaError> {
        let (parts, body) = request.into_parts();
        let body_bytes = lambda_body_into_bytes(body);

        {
            let body_size = body_bytes.len();
            let is_compressed = parts
                .headers
                .get("content-encoding")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("gzip"));

            info!(
                method = %parts.method,
                uri = %parts.uri,
                body_size_bytes = body_size,
                body_compressed = is_compressed,
                "Received incoming HTTP request"
            );
        }

        // if let Err(err) = ensure_session_header(&mut parts.headers, &self.state).await {
        //     return Ok(snowflake_error_response(&err));
        // }

        let mut axum_request = to_axum_request(parts, body_bytes);
        if let Some(addr) = extract_socket_addr(axum_request.headers()) {
            axum_request.extensions_mut().insert(ConnectInfo(addr));
        }

        let response = self
            .router
            .clone()
            .oneshot(axum_request)
            .await
            .expect("Router service should be infallible");

        let lambda_response = from_axum_response(response).await?;

        // Record response status in the current span
        tracing::Span::current().record("http.status_code", lambda_response.status().as_u16());

        Ok(lambda_response)
    }
}

fn to_axum_request(parts: http::request::Parts, body: Vec<u8>) -> http::Request<AxumBody> {
    http::Request::from_parts(parts, AxumBody::from(body))
}

fn lambda_body_into_bytes(body: LambdaBody) -> Vec<u8> {
    match body {
        LambdaBody::Empty => Vec::new(),
        LambdaBody::Text(text) => text.into_bytes(),
        LambdaBody::Binary(data) => data,
    }
}

async fn from_axum_response(
    response: axum::response::Response,
) -> Result<Response<LambdaBody>, LambdaError> {
    let (parts, body) = response.into_parts();
    let bytes = body
        .collect()
        .await
        .map_err(|err| -> LambdaError { Box::new(err) })?
        .to_bytes();

    let body_size = bytes.len();
    let is_compressed = parts
        .headers
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("gzip"));

    info!(
        status = %parts.status,
        body_size_bytes = body_size,
        body_compressed = is_compressed,
        "Sending HTTP response"
    );

    let mut lambda_response = Response::new(LambdaBody::Binary(bytes.to_vec()));
    *lambda_response.status_mut() = parts.status;
    *lambda_response.headers_mut() = parts.headers;
    Ok(lambda_response)
}

fn extract_socket_addr(headers: &HeaderMap) -> Option<SocketAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .and_then(|ip| ip.trim().parse::<IpAddr>().ok())
        .map(|ip| SocketAddr::new(ip, 0))
}

#[allow(clippy::expect_used, clippy::redundant_closure_for_method_calls)]
fn init_tracing_and_logs(config: &EnvConfig) -> SdkTracerProvider {
    let exporter = if config.otel_grpc {
        // Initialize OTLP exporter using gRPC (Tonic)
        opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .build()
            .expect("Failed to create OTLP gRPC exporter")
    } else {
        // Initialize OTLP exporter using HTTP
        opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .build()
            .expect("Failed to create OTLP HTTP exporter")
    };

    let resource = Resource::builder().with_service_name("embucket-lambda-api").build();

    let tracing_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(resource)
        .build();

    let targets_with_level =
        |targets: &[&'static str], level: LevelFilter| -> Vec<(&str, LevelFilter)> {
            // let default_log_targets: Vec<(String, LevelFilter)> =
            targets.iter().map(|t| ((*t), level)).collect()
        };

    let registry = tracing_subscriber::registry()
        // Telemetry filtering
        .with(
            tracing_opentelemetry::OpenTelemetryLayer::new(tracing_provider.tracer("embucket"))
                .with_level(true)
                .with_filter(
                    Targets::default()
                        .with_targets(targets_with_level(&DISABLED_TARGETS, LevelFilter::OFF))
                        .with_default(config.tracing_level.parse().unwrap_or(tracing::Level::INFO)),
                ),
        );
    // Logs filtering
    // fmt::layer has different types for json vs plain
    if config.log_format == "json" {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(false)
                    .with_ansi(false)
                    .with_current_span(false)
                    .with_span_list(false)
                    .without_time(),
            )
            .with(EnvFilter::new(config.log_filter.clone()))
            .init();
    } else {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_ansi(std::io::stdout().is_terminal())
                    .with_span_events(
                        tracing_subscriber::fmt::format::FmtSpan::ENTER
                            | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
                    ),
            )
            .with(EnvFilter::new(config.log_filter.clone()))
            .init();
    }

    tracing_provider
}