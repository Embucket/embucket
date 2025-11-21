mod config;
mod metastore_config;

use crate::config::EnvConfig;
use crate::metastore_config::MetastoreBootstrapConfig;
use api_snowflake_rest::server::layer::require_auth;
use api_snowflake_rest::server::router::{create_auth_router, create_router};
use api_snowflake_rest::server::server_models::Config as SnowflakeServerConfig;
use api_snowflake_rest::server::state::AppState;
use api_snowflake_rest_sessions::session::{SESSION_EXPIRATION_SECONDS, SessionStore};
use api_snowflake_rest_sessions::layer::Host;
use axum::body::Body as AxumBody;
use axum::extract::connect_info::ConnectInfo;
use axum::{Router, middleware};
use axum::Extension;
use catalog_metastore::InMemoryMetastore;
use executor::service::CoreExecutionService;
use http::HeaderMap;
use http_body_util::BodyExt;
use lambda_http::{Body as LambdaBody, Error as LambdaError, Request, Response, run, service_fn};
use std::io::IsTerminal;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::time::Duration;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::compression::CompressionLayer;
use tower_http::decompression::RequestDecompressionLayer;
use tracing::{error, info};

type InitResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    init_tracing();

    let env_config = EnvConfig::from_env();
    info!(
        data_format = %env_config.data_format,
        max_concurrency = env_config.max_concurrency_level,
        query_timeout_secs = env_config.query_timeout_secs,
        mem_pool_type = ?env_config.mem_pool_type,
        mem_pool_size_mb = ?env_config.mem_pool_size_mb,
        disk_pool_size_mb = ?env_config.disk_pool_size_mb,
        bootstrap_default_entities = env_config.bootstrap_default_entities,
        metastore_config = env_config.metastore_config.as_ref().map(|p| p.display().to_string()),
        "Loaded Lambda configuration"
    );

    let app = Arc::new(LambdaApp::initialize(env_config).await.map_err(|err| {
        error!(error = %err, "Failed to initialize Lambda services");
        err
    })?);

    run(service_fn(move |event: Request| {
        let app = Arc::clone(&app);
        async move { app.handle_event(event).await }
    }))
    .await
}

struct LambdaApp {
    router: Router,
    state: AppState,
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
        let metastore = Arc::new(InMemoryMetastore::new());

        if let Some(config_path) = &config.metastore_config {
            info!(
                path = %config_path.display(),
                "Bootstrapping metastore from config"
            );
            let bootstrap_cfg = MetastoreBootstrapConfig::load(config_path.as_path())
                .await
                .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?;
            bootstrap_cfg
                .apply(metastore.clone())
                .await
                .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?;
        }

        let execution_svc = Arc::new(
            CoreExecutionService::new(metastore, Arc::new(execution_cfg))
                .await
                .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?,
        );

        let session_store = SessionStore::new(execution_svc.clone());
        tokio::spawn(async move {
            session_store
                .continuously_delete_expired(Duration::from_secs(SESSION_EXPIRATION_SECONDS))
                .await;
        });

        info!("Initialized Lambda Snowflake REST services");

        let state = AppState {
            execution_svc,
            config: snowflake_cfg,
        };

        let compression_layer = ServiceBuilder::new()
            .layer(CompressionLayer::new())
            .layer(RequestDecompressionLayer::new());

        let snowflake_router = create_router()
            .with_state(state.clone())
            .layer(compression_layer.clone())
            .layer(Extension(Host(String::default())))
            .layer(middleware::from_fn_with_state(state.clone(), require_auth));
        let snowflake_auth_router = create_auth_router()
            .with_state(state.clone())
            .layer(compression_layer)
            .layer(Extension(Host(String::default())));
        let router = Router::new().merge(snowflake_router.merge(snowflake_auth_router));

        Ok(Self { router, state })
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

// #[tracing::instrument(name = "ensure_session_header", skip_all)]
// async fn ensure_session_header(
//     headers: &mut HeaderMap,
//     state: &AppState,
// ) -> Result<(), SnowflakeError> {
//     if let Some(token) = extract_token_from_auth(headers) {
//         ensure_session(state, &token).await
//     } else {
//         let session_id = Uuid::new_v4().to_string();
//         state.execution_svc.create_session(&session_id).await?;
//         let header_value = HeaderValue::from_str(&format!("Snowflake Token=\"{session_id}\""))
//             .map_err(|_| SnowflakeError::invalid_auth_data())?;
//         headers.insert(AUTHORIZATION, header_value);
//         Ok(())
//     }
// }
//
// #[tracing::instrument(name = "ensure_session", skip(state), fields(session_id = %session_id))]
// async fn ensure_session(state: &AppState, session_id: &str) -> Result<(), SnowflakeError> {
//     if !state
//         .execution_svc
//         .update_session_expiry(session_id)
//         .await?
//     {
//         let _ = state.execution_svc.create_session(session_id).await?;
//     }
//     Ok(())
// }
//
// fn snowflake_error_response(err: &SnowflakeError) -> Response<LambdaBody> {
//     let (status, axum::Json(body)) = err.prepare_response();
//     let payload =
//         serde_json::to_string(&body).unwrap_or_else(|_| "{\"success\":false}".to_string());
//     let body_preview = payload.clone();
//     let mut response = Response::new(LambdaBody::Text(payload));
//     *response.status_mut() = status;
//     response
//         .headers_mut()
//         .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
//     info!(
//         status = %response.status(),
//         headers = ?response.headers(),
//         body = %body_preview,
//         "Sending HTTP error response"
//     );
//     response
// }

fn init_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let emit_ansi = std::io::stdout().is_terminal();

    // Use json format if requested via env var, otherwise use pretty format with span events
    let format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    if format == "json" {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_ansi(false)
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_ansi(emit_ansi)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::ENTER
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .try_init();
    }
}
