mod config;
mod tracing_setup;

use crate::config::EnvConfig;
use crate::tracing_setup::{init_tracing, trace_flusher};
use api_snowflake_rest::server::core_state::CoreState;
use api_snowflake_rest::server::core_state::MetastoreConfig;
use api_snowflake_rest::server::layer::require_auth;
use api_snowflake_rest::server::make_snowflake_router;
use api_snowflake_rest::server::server_models::RestApiConfig as SnowflakeServerConfig;
use api_snowflake_rest::server::state::AppState;
use api_snowflake_rest_sessions::session::SESSION_EXPIRATION_SECONDS;
use axum::body::Body as AxumBody;
use axum::extract::connect_info::ConnectInfo;
use catalog_metastore::metastore_settings_config::MetastoreSettingsConfig;
use axum::middleware::from_fn_with_state;
use axum::{Router, middleware};
use http::HeaderMap;
use http_body_util::BodyExt;
use lambda_http::{
    Body as LambdaBody, Error as LambdaError, Request, Response, service_fn, tracing,
};
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tower::ServiceExt;
use tracing::{error, info};

cfg_if::cfg_if! {
    if #[cfg(feature = "streaming")] {
        use lambda_http::run_with_streaming_response as run;
    } else {
        use lambda_http::run;
    }
}

type InitResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    //Has `Arc` inside
    let tracer_provider = init_tracing();

    // tracing::init_default_subscriber();

    let env_config = EnvConfig::from_env();
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

    let app = Arc::new(
        LambdaApp::initialize(env_config, tracer_provider.clone())
            .await
            .map_err(|err| {
                error!(error = %err, "Failed to initialize Lambda services");
                err
            })?,
    );

    run(service_fn(move |event: Request| {
        let app = Arc::clone(&app);
        async move { app.handle_event(event).await }
    }))
    .await?;

    tracer_provider.shutdown()?;

    Ok(())
}

struct LambdaApp {
    router: Router,
}

impl LambdaApp {
    #[tracing::instrument(name = "lambda_app_initialize", skip_all, fields(
        data_format = %config.data_format,
        max_concurrency = config.max_concurrency_level
    ))]
    async fn initialize(config: EnvConfig, trace_provider: SdkTracerProvider) -> InitResult<Self> {
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
        let router = make_snowflake_router(appstate)
            // .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(middleware::from_fn_with_state(
                trace_provider.clone(),
                trace_flusher,
            ));
        info!("Initialized Lambda Snowflake REST services");

        Ok(Self { router })
    }

    #[tracing::instrument(name = "lambda_handle_event", skip_all, fields(
        http.method = %request.method(),
        http.uri = %request.uri(),
        http.request_id = tracing::field::Empty,
        http.status_code = tracing::field::Empty,
        yes = tracing::field::Empty,
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
        tracing::Span::current().record("yes", "no");

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
