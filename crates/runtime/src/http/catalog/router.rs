use crate::http::state::AppState;
use axum::routing::{delete, get, post};
use axum::Router;

#[allow(clippy::wildcard_imports)]
use crate::http::catalog::handlers::*;

pub fn create_router() -> Router<AppState> {
    let table_router: Router<AppState> = Router::new()
        .route("/", post(create_table))
        .route("/", get(list_tables))
        .route("/{table}", get(get_table))
        .route("/{table}", delete(delete_table))
        .route("/{table}", post(commit_table))
        .route("/{table}/metrics", post(report_metrics));

    // only one endpoint is defined for the catalog implementation to work
    // we don't actually have functionality for views yet
    let view_router: Router<AppState> = Router::new().route("/", get(list_views));

    let ns_router = Router::new()
        .route("/", get(list_namespaces))
        .route("/", post(create_namespace))
        .route("/{namespace}", get(get_namespace))
        .route("/{namespace}", delete(delete_namespace))
        .route("/{namespace}/register", post(register_table))
        .nest("/{namespace}/tables", table_router)
        .nest("/{namespace}/views", view_router);

    // Iceberg clients do not prefix config fetch RPC call
    // and do prefix (with whatever prefix returned by config fetch) all other RPC calls
    // We return warehouse id as a part of config fetch response and thus expecting it
    // as part of URL (:wid)

    Router::new()
        .route("/v1/config", get(get_config))
        .nest("/v1/{wid}/namespaces", ns_router)
}
