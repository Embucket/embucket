use super::TEST_JWT_SECRET;
use crate::server::core_state::CoreState;
use crate::server::core_state::MetastoreConfig;
use crate::server::make_snowflake_router;
use crate::server::server_models::RestApiConfig;
use crate::server::state::AppState;
use catalog_metastore::metastore_settings_config::MetastoreSettingsConfig;
use executor::utils::Config as UtilsConfig;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Builder;
use tracing_subscriber::fmt::format::FmtSpan;

static INIT: std::sync::Once = std::sync::Once::new();

#[allow(clippy::expect_used)]
#[must_use]
pub fn rest_default_cfg(data_format: &str) -> RestApiConfig {
    RestApiConfig::new(data_format, TEST_JWT_SECRET.to_string())
        .expect("Failed to create server config")
        .with_demo_credentials("embucket".to_string(), "embucket".to_string())
}

#[allow(clippy::expect_used)]
#[must_use]
pub fn executor_default_cfg() -> UtilsConfig {
    UtilsConfig::default().with_max_concurrency_level(2)
}

#[must_use]
pub fn metastore_default_settings_cfg() -> MetastoreSettingsConfig {
    MetastoreSettingsConfig::default()
        .with_object_store_connect_timeout(1)
        .with_object_store_timeout(1)
}

#[allow(clippy::expect_used)]
pub fn run_test_rest_api_server(
    rest_cfg: Option<RestApiConfig>,
    executor_cfg: Option<UtilsConfig>,
    metastore_settings_cfg: Option<MetastoreSettingsConfig>,
    metastore_cfg: MetastoreConfig,
) -> SocketAddr {
    let rest_cfg = rest_cfg.unwrap_or_else(|| rest_default_cfg("json"));
    let executor_cfg = executor_cfg.unwrap_or_else(executor_default_cfg);
    let metastore_settings_cfg =
        metastore_settings_cfg.unwrap_or_else(metastore_default_settings_cfg);

    let server_cond = Arc::new((Mutex::new(false), Condvar::new())); // Shared state with a condition 
    let server_cond_clone = Arc::clone(&server_cond);

    let listener = TcpListener::bind("0.0.0.0:0").expect("Failed to bind to address");
    let addr = listener.local_addr().expect("Failed to get local address");

    // Start a new thread for the server
    let _handle = std::thread::spawn(move || {
        // Create the Tokio runtime
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        // Start the Axum server
        rt.block_on(async {
            let () = run_test_rest_api_server_with_config(
                rest_cfg,
                executor_cfg,
                metastore_settings_cfg,
                metastore_cfg,
                listener,
                server_cond_clone,
            )
            .await;
        });
    });
    // Note: Not joining thread as
    // We are not interested in graceful thread termination, as soon out tests passed.

    let (lock, cvar) = &*server_cond;
    let timeout_duration = std::time::Duration::from_secs(1);

    // Lock the mutex and wait for notification with timeout
    let notified = lock.lock().expect("Failed to lock mutex");
    let result = cvar
        .wait_timeout(notified, timeout_duration)
        .expect("Failed to wait for server start");

    // Check if notified or timed out
    if *result.0 {
        tracing::info!("Test server is up and running.");
        thread::sleep(Duration::from_millis(10));
    } else {
        tracing::error!("Timeout occurred while waiting for server start.");
    }

    addr
}

fn setup_tracing() {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_sdk::Resource;
    use opentelemetry_sdk::trace::BatchSpanProcessor;
    use opentelemetry_sdk::trace::SdkTracerProvider;
    use tracing_subscriber::filter::{LevelFilter, Targets};
    use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

    const DISABLED_TARGETS: [&str; 1] = ["h2"];

    INIT.call_once(|| {
        // Initialize OTLP exporter using gRPC (Tonic)
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .build()
            .expect("Failed to create OTLP exporter");

        let resource = Resource::builder().with_service_name("Em").build();

        // Since BatchSpanProcessor and BatchSpanProcessorAsyncRuntime are not compatible with each other
        // we just create TracerProvider with different span processors
        let tracing_provider = SdkTracerProvider::builder()
            .with_span_processor(BatchSpanProcessor::builder(exporter).build())
            .with_resource(resource)
            .build();

        let targets_with_level =
            |targets: &[&'static str], level: LevelFilter| -> Vec<(&str, LevelFilter)> {
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
                            .with_default(LevelFilter::TRACE),
                    ),
            )
            // Logs filtering
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(
                        std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("traces.log")
                            .expect("Failed to open traces.log"),
                    )
                    .with_ansi(false)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_span_events(FmtSpan::NONE)
                    .json()
                    .with_level(true)
                    .with_filter(
                        Targets::default()
                            .with_targets(targets_with_level(&DISABLED_TARGETS, LevelFilter::OFF))
                            .with_default(LevelFilter::TRACE),
                    ),
            );

        registry.init();
        opentelemetry::global::set_tracer_provider(tracing_provider);
    });
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
pub async fn run_test_rest_api_server_with_config(
    snowflake_rest_cfg: RestApiConfig,
    execution_cfg: UtilsConfig,
    metastore_settings_cfg: MetastoreSettingsConfig,
    metastore_cfg: MetastoreConfig,
    listener: std::net::TcpListener,
    server_cond: Arc<(Mutex<bool>, Condvar)>,
) {
    let addr = listener.local_addr().unwrap();

    setup_tracing();
    tracing::info!("Starting server at {addr}");

    let core_state = CoreState::new(
        execution_cfg,
        snowflake_rest_cfg,
        metastore_settings_cfg,
        metastore_cfg,
    )
    .await
    .expect("Core state creation error");

    let app = make_snowflake_router(AppState::from(&core_state))
        .into_make_service_with_connect_info::<SocketAddr>();

    // Lock the mutex and set the notification flag
    {
        let (lock, cvar) = &*server_cond;
        let mut notify_server_started = lock.lock().unwrap();
        *notify_server_started = true; // Set notification
        cvar.notify_one(); // Notify the waiting thread
    }

    tracing::info!("Server ready at {addr}");

    // Serve the application
    axum_server::from_tcp(listener).serve(app).await.unwrap();
}
