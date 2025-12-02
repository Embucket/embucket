use super::handlers::{abort, login, query, session};
use super::layer::require_auth;
use super::state::AppState;
use api_snowflake_rest_sessions::layer::Host;
use axum::middleware;
use axum::routing::post;
use axum::{Extension, Router};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::decompression::RequestDecompressionLayer;

pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/session/v1/login-request", post(login))
        .route("/session", post(session))
}

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/queries/v1/query-request", post(query))
        .route("/queries/v1/abort-request", post(abort))
}

pub fn make_snowflake_router(app_state: AppState) -> Router {
    let compression_layer = ServiceBuilder::new()
        .layer(CompressionLayer::new())
        .layer(RequestDecompressionLayer::new());

    let snowflake_router = create_router()
        .with_state(app_state.clone())
        .layer(compression_layer.clone())
        .layer(Extension(Host(String::default())))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_auth,
        ));
    let snowflake_auth_router = create_auth_router()
        .with_state(app_state)
        .layer(compression_layer)
        .layer(Extension(Host(String::default())));

    snowflake_router.merge(snowflake_auth_router)
}
