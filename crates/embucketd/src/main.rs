// Set this clippy directive to suppress clippy::needless_for_each warnings
// until following issue will be fixed https://github.com/juhaku/utoipa/issues/1420
#![allow(clippy::needless_for_each)]
pub(crate) mod cli;
pub(crate) mod helpers;
pub(crate) mod layers;

use api_snowflake_rest::server::core_state::CoreState;
use api_snowflake_rest::server::core_state::MetastoreConfig;
use api_snowflake_rest::server::make_snowflake_router;
use api_snowflake_rest::server::server_models::RestApiConfig;
use api_snowflake_rest::server::state::AppState;
use api_snowflake_rest_sessions::session::SESSION_EXPIRATION_SECONDS;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    routing::{get, post},
};
use catalog_metastore::global_settings::GlobalSettings;
use clap::Parser;
use dotenv::dotenv;
use executor::service::{ExecutionService, TIMEOUT_SIGNAL_INTERVAL_SECONDS};
use executor::utils::Config as ExecutionConfig;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::runtime::TokioCurrentThread;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor as BatchSpanProcessorAsyncRuntime;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::filter::{FilterExt, LevelFilter, Targets, filter_fn};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "alloc-tracing")]
mod alloc_tracing {
    pub use crate::layers::AllocLogLayer;
    pub use tracing_allocations::{TRACE_ALLOCATOR, TracingAllocator};

    #[global_allocator]
    static ALLOCATOR: TracingAllocator<tikv_jemallocator::Jemalloc> =
        TracingAllocator::new(tikv_jemallocator::Jemalloc);
}

#[cfg(not(feature = "alloc-tracing"))]
#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

const DISABLED_TARGETS: [&str; 2] = ["h2", "aws_smithy_runtime"];

#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::too_many_lines
)]
fn main() {
    dotenv().ok();

    let opts = cli::CliOpts::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .on_thread_start({
            move || {
                #[cfg(feature = "alloc-tracing")]
                if opts.alloc_tracing.unwrap_or(false) {
                    alloc_tracing::TRACE_ALLOCATOR.with(|cell| *cell.borrow_mut() = true);
                }
            }
        })
        .build()
        .expect("build tokio runtime");

    rt.block_on(async move {
        let tracing_provider = setup_tracing(&opts);

        if let Err(e) = async_main(opts, tracing_provider).await {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    });
}

#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
async fn async_main(
    opts: cli::CliOpts,
    tracing_provider: SdkTracerProvider,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_format = opts
        .data_format
        .clone()
        .unwrap_or_else(|| "json".to_string());
    let snowflake_rest_cfg = RestApiConfig::new(&data_format, opts.jwt_secret())
        .expect("Failed to create snowflake config")
        .with_demo_credentials(
            opts.auth_demo_user.clone().unwrap(),
            opts.auth_demo_password.clone().unwrap(),
        );

    let execution_cfg = ExecutionConfig {
        embucket_version: "0.1.0".to_string(),
        sql_parser_dialect: opts.sql_parser_dialect.clone(),
        query_timeout_secs: opts.query_timeout_secs,
        max_concurrency_level: opts.max_concurrency_level,
        mem_pool_type: opts.mem_pool_type,
        mem_pool_size_mb: opts.mem_pool_size_mb,
        mem_enable_track_consumers_pool: opts.mem_enable_track_consumers_pool,
        disk_pool_size_mb: opts.disk_pool_size_mb,
        max_concurrent_table_fetches: opts.max_concurrent_table_fetches,
        #[cfg(not(feature = "rest-catalog"))]
        aws_sdk_operation_timeout_secs: opts.aws_sdk_operation_timeout_secs,
        #[cfg(not(feature = "rest-catalog"))]
        aws_sdk_operation_attempt_timeout_secs: opts.aws_sdk_operation_attempt_timeout_secs,
        #[cfg(not(feature = "rest-catalog"))]
        aws_sdk_connect_timeout_secs: opts.aws_sdk_connect_timeout_secs,
        iceberg_create_table_timeout_secs: opts.iceberg_create_table_timeout_secs,
        iceberg_catalog_timeout_secs: opts.iceberg_catalog_timeout_secs,
    };

    GlobalSettings::new()
        .with_object_store_timeout(opts.object_store_timeout_secs)
        .with_object_store_connect_timeout(opts.object_store_connect_timeout_secs)
        .initialize()
        .expect("Failed to initialize global settings");

    let host = opts.host.clone().unwrap();
    let port = opts.port.unwrap();

    let metastore_cfg = if let Some(config_path) = &opts.metastore_config {
        MetastoreConfig::ConfigPath(config_path.clone())
    } else {
        MetastoreConfig::None
    };
    let core_state = CoreState::new(execution_cfg, snowflake_rest_cfg, metastore_cfg)
        .await
        .expect("Core state creation error");

    core_state
        .with_session_timeout(tokio::time::Duration::from_secs(SESSION_EXPIRATION_SECONDS))?;

    let appstate = AppState::from(&core_state);
    let snowflake_router = make_snowflake_router(appstate);

    let execution_svc = core_state.executor.clone();
    // --- OpenAPI specs ---
    let swagger = SwaggerUi::new("/").url("/openapi.json", ApiDoc::openapi());

    let router = Router::new()
        .merge(snowflake_router)
        .merge(swagger)
        .route("/health", get(|| async { Json("OK") }))
        .route("/telemetry/send", post(|| async { Json("OK") }))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(1200),
        ))
        .layer(CatchPanicLayer::new())
        .into_make_service_with_connect_info::<SocketAddr>();

    let web_addr = helpers::resolve_ipv4(format!("{host}:{port}"))
        .expect("Failed to resolve web server address");
    let listener = tokio::net::TcpListener::bind(web_addr)
        .await
        .expect("Failed to bind to address");
    let addr = listener.local_addr().expect("Failed to get local address");
    tracing::info!(%addr, "Listening on http");
    let timeout = opts.timeout.unwrap();
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(execution_svc, timeout))
        .await
        .expect("Failed to start server");

    tracing_provider
        .shutdown()
        .expect("TracerProvider should shutdown successfully");

    Ok(())
}

#[allow(clippy::expect_used, clippy::redundant_closure_for_method_calls)]
fn setup_tracing(opts: &cli::CliOpts) -> SdkTracerProvider {
    // Initialize OTLP exporter using gRPC (Tonic)
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create OTLP exporter");

    let resource = Resource::builder().with_service_name("Em").build();

    // Since BatchSpanProcessor and BatchSpanProcessorAsyncRuntime are not compatible with each other
    // we just create TracerProvider with different span processors
    let tracing_provider = match opts.tracing_span_processor {
        cli::TracingSpanProcessor::BatchSpanProcessor => SdkTracerProvider::builder()
            .with_span_processor(BatchSpanProcessor::builder(exporter).build())
            .with_resource(resource)
            .build(),
        cli::TracingSpanProcessor::BatchSpanProcessorExperimentalAsyncRuntime => {
            SdkTracerProvider::builder()
                .with_span_processor(
                    BatchSpanProcessorAsyncRuntime::builder(exporter, TokioCurrentThread).build(),
                )
                .with_resource(resource)
                .build()
        }
    };

    let targets_with_level =
        |targets: &[&'static str], level: LevelFilter| -> Vec<(&str, LevelFilter)> {
            // let default_log_targets: Vec<(String, LevelFilter)> =
            targets.iter().map(|t| ((*t), level)).collect()
        };

    // Memory allocations
    #[cfg(feature = "alloc-tracing")]
    let alloc_layer =
        alloc_tracing::AllocLogLayer::write_to_file("./alloc.log").expect("open alloc log");

    #[cfg(feature = "alloc-tracing")]
    {
        let alloc_flusher = Arc::new(alloc_layer.clone());
        alloc_flusher.spawn_flusher(std::time::Duration::from_secs(1));
    }

    let registry = tracing_subscriber::registry()
        // Telemetry filtering
        .with(
            tracing_opentelemetry::OpenTelemetryLayer::new(tracing_provider.tracer("embucket"))
                .with_level(true)
                .with_filter(
                    Targets::default()
                        .with_targets(targets_with_level(&DISABLED_TARGETS, LevelFilter::OFF))
                        .with_default(opts.tracing_level.clone()),
                ),
        )
        // Logs filtering
        .with({
            let fmt_filter = match std::env::var("RUST_LOG") {
                Ok(val) => match val.parse::<Targets>() {
                    Ok(log_targets_from_env) => log_targets_from_env,
                    Err(err) => {
                        eprintln!("Failed to parse RUST_LOG: {err:?}");
                        Targets::default()
                            .with_targets(targets_with_level(&DISABLED_TARGETS, LevelFilter::OFF))
                            .with_default(LevelFilter::DEBUG)
                    }
                },
                _ => Targets::default()
                    .with_targets(targets_with_level(&DISABLED_TARGETS, LevelFilter::OFF))
                    .with_default(LevelFilter::INFO),
            };
            // Skip memory allocations spans
            let spans_always = filter_fn(|meta| meta.is_span());
            let not_alloc_event = filter_fn(|meta| {
                meta.target() != "alloc" && meta.target() != "tracing_allocations"
            });

            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_span_events(FmtSpan::NONE)
                .json()
                .with_filter(spans_always.or(not_alloc_event.and(fmt_filter)))
        });

    // Memory allocations layer
    #[cfg(feature = "alloc-tracing")]
    let registry = registry.with(alloc_layer.with_filter(filter_fn(|meta| {
        meta.target() == "tracing_allocations" || meta.target() == "alloc"
    })));
    registry.init();
    tracing_provider
}

/// This func will wait for a signal to shutdown the service.
/// It will wait for either a Ctrl+C signal or a SIGTERM signal.
///
/// # Panics
/// If the function fails to install the signal handler, it will panic.
#[allow(
    clippy::expect_used,
    clippy::redundant_pub_crate,
    clippy::cognitive_complexity
)]
async fn shutdown_signal(execution_svc: Arc<dyn ExecutionService>, timeout: u64) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    let timeout = execution_svc.timeout_signal(
        tokio::time::Duration::from_secs(TIMEOUT_SIGNAL_INTERVAL_SECONDS),
        tokio::time::Duration::from_secs(timeout),
    );

    tokio::select! {
        () = ctrl_c => {
            tracing::warn!("Ctrl+C received, starting graceful shutdown");
        },
        () = terminate => {
            tracing::warn!("SIGTERM received, starting graceful shutdown");
        },
        () = timeout => {
            tracing::warn!("No sessions in use & no running queries - timeout, starting graceful shutdown");
        }
    }

    tracing::warn!("signal received, starting graceful shutdown");
}

// TODO: Fix OpenAPI spec generation
#[derive(OpenApi)]
#[openapi()]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use api_snowflake_rest_sessions::session::SessionStore;
    use executor::models::QueryContext;
    use executor::service::ExecutionService;
    use executor::service::make_test_execution_svc;
    use executor::session::to_unix;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use time::OffsetDateTime;

    #[tokio::test]
    #[allow(clippy::expect_used, clippy::too_many_lines)]
    async fn test_timeout_signal() {
        let execution_svc = make_test_execution_svc().await;

        let df_session_id = "fasfsafsfasafsass".to_string();
        let user_session = execution_svc
            .create_session(&df_session_id)
            .await
            .expect("Failed to create a session");

        execution_svc
            .query(&df_session_id, "SELECT SLEEP(5)", QueryContext::default())
            .await
            .expect("Failed to execute query (session deleted)");

        user_session
            .expiry
            .store(to_unix(OffsetDateTime::now_utc()), Ordering::Relaxed);

        let session_store = SessionStore::new(execution_svc.clone());

        tokio::task::spawn({
            let session_store = session_store.clone();
            async move {
                session_store
                    .continuously_delete_expired(Duration::from_secs(1))
                    .await;
            }
        });

        let timeout = execution_svc.timeout_signal(Duration::from_secs(1), Duration::from_secs(3));
        tokio::select! {
            () = timeout => {
                tracing::warn!("No sessions in use & no running queries - timeout, starting graceful shutdown");
            }
        }
    }
}
