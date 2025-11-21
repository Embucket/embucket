use super::state::AppState;
use crate::models::{
    AbortRequestBody, JsonResponse, LoginRequestBody, LoginResponse, QueryRequest, QueryRequestBody,
};
use crate::server::error::Result;
use crate::server::logic::{handle_login_request, handle_query_request};
use api_snowflake_rest_sessions::DFSessionId;
use api_snowflake_rest_sessions::layer::Host;
use axum::Json;
use axum::extract::{ConnectInfo, Query, State};
use executor::RunningQueryId;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize)]
pub struct SessionQueryParams {
    #[serde(default)]
    delete: bool,
    #[serde(rename = "requestId", alias = "request_id")]
    request_id: Option<String>,
    #[serde(rename = "request_guid", alias = "requestGuid")]
    request_guid: Option<String>,
}

#[tracing::instrument(name = "api_snowflake_rest::login", level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn login(
    Host(host): Host,
    State(state): State<AppState>,
    Json(login_request): Json<LoginRequestBody>,
) -> Result<Json<LoginResponse>> {
    let response = handle_login_request(&state, host, login_request.data).await?;
    Ok(Json(response))
}

#[tracing::instrument(
    name = "api_snowflake_rest::query",
    level = "debug",
    skip(state),
    fields(query_id),
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
    let query_id = state
        .execution_svc
        .locate_query_id(RunningQueryId::ByRequestId(request_id, sql_text))?;
    state.execution_svc.abort(query_id)?;
    Ok(Json(serde_json::value::Value::Null))
}

#[tracing::instrument(
    name = "api_snowflake_rest::session",
    level = "debug",
    skip(state, query_params),
    fields(session_id, request_id, request_guid, delete),
    err,
    ret(level = tracing::Level::TRACE)
)]
pub async fn session(
    DFSessionId(session_id): DFSessionId,
    State(state): State<AppState>,
    Query(query_params): Query<SessionQueryParams>,
) -> Result<Json<serde_json::value::Value>> {
    let SessionQueryParams {
        delete,
        request_id,
        request_guid,
    } = query_params;

    let span = tracing::Span::current();
    span.record("session_id", session_id.as_str());
    if let Some(ref request_id) = request_id {
        span.record("request_id", request_id.as_str());
    }
    if let Some(ref request_guid) = request_guid {
        span.record("request_guid", request_guid.as_str());
    }
    span.record("delete", delete);

    if delete {
        state.execution_svc.delete_session(&session_id).await?;
    } else {
        tracing::debug!("Session endpoint called without delete flag; ignoring request");
    }

    Ok(Json(serde_json::value::Value::Null))
}
