use super::state::AppState;
use crate::models::{
    AbortRequestBody, JsonResponse, LoginRequestBody, LoginResponse, QueryRequest, QueryRequestBody,
};
use crate::server::error::Result;
use crate::server::logic::{handle_login_request, handle_query_request};
use api_snowflake_rest_sessions::DFSessionId;
use axum::Json;
use axum::extract::{ConnectInfo, Query, State};
use executor::RunningQueryId;
use std::net::SocketAddr;

#[tracing::instrument(name = "api_snowflake_rest::login", level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn login(
    State(state): State<AppState>,
    Json(login_request): Json<LoginRequestBody>,
) -> Result<Json<LoginResponse>> {
    let response = handle_login_request(&state, login_request.data).await?;
    Ok(Json(response))
}

#[tracing::instrument(
    name = "api_snowflake_rest::query",
    level = "debug",
    skip(state),
    fields(query_id, query_uuid),
    err,
    ret(level = tracing::Level::TRACE),
)]
pub async fn query(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    DFSessionId(session_id): DFSessionId,
    State(state): State<AppState>,
    Query(query): Query<QueryRequest>,
    Json(query_body): Json<QueryRequestBody>,
) -> Result<Json<JsonResponse>> {
    let response = handle_query_request(
        &state,
        &session_id,
        query,
        query_body,
        Option::from(addr.ip().to_string()),
    )
    .await?;

    Ok(Json(response))
}

#[tracing::instrument(name = "api_snowflake_rest::abort", level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn abort(
    State(state): State<AppState>,
    Json(AbortRequestBody {
        sql_text,
        request_id,
    }): Json<AbortRequestBody>,
) -> Result<Json<serde_json::value::Value>> {
    state
        .execution_svc
        .abort_query(RunningQueryId::ByRequestId(request_id, sql_text))?;
    Ok(Json(serde_json::value::Value::Null))
}
