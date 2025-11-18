use super::state::AppState;
use crate::models::{
    AbortRequestBody, JsonResponse, LoginRequestBody, LoginRequestData, LoginResponse,
    LoginResponseData, QueryRequest, QueryRequestBody,
};
use crate::server::error::{self as api_snowflake_rest_error, Result};
use crate::server::helpers::handle_query_ok_result;
use api_snowflake_rest_sessions::DFSessionId;
use axum::Json;
use axum::extract::{ConnectInfo, Query, State};
use executor::RunningQueryId;
use executor::models::QueryContext;
use std::net::SocketAddr;
use uuid::Uuid;

#[tracing::instrument(name = "api_snowflake_rest::login", level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn login(
    State(state): State<AppState>,
    // Query(_query_params): Query<LoginRequestQueryParams>,
    Json(LoginRequestBody {
        data:
            LoginRequestData {
                login_name,
                password,
                ..
            },
    }): Json<LoginRequestBody>,
) -> Result<Json<LoginResponse>> {
    if login_name != *state.config.auth.demo_user || password != *state.config.auth.demo_password {
        return api_snowflake_rest_error::InvalidAuthDataSnafu.fail()?;
    }

    let session_id = Uuid::new_v4().to_string();

    let _ = state.execution_svc.create_session(&session_id).await?;

    Ok(Json(LoginResponse {
        data: Option::from(LoginResponseData { token: session_id }),
        success: true,
        message: Option::from("successfully executed".to_string()),
    }))
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
    Json(QueryRequestBody {
        sql_text,
        async_exec,
    }): Json<QueryRequestBody>,
) -> Result<Json<JsonResponse>> {
    let serialization_format = state.config.dbt_serialization_format;
    let query_context = QueryContext::default()
        .with_ip_address(addr.ip().to_string())
        .with_request_id(query.request_id);

    if async_exec {
        return api_snowflake_rest_error::NotImplementedSnafu.fail();
    }

    // find running query by request_id
    let session = state.execution_svc.get_session(&session_id).await?;
    let running_query_id = RunningQueryId::ByRequestId(query.request_id, sql_text.clone());
    let query_id_res = session.running_queries.locate_query_id(running_query_id.clone());

    let (result, query_id) = if query.retry_count.unwrap_or_default() > 0 && let Ok(query_id ) = query_id_res {
        let result = state
            .execution_svc
            .wait_submitted_query_result(running_query_id)
            .await?;
        (result, query_id)
    } else {
        let query_id = query_context.query_id;
        let result = state
        .execution_svc
        .query(&session_id, &sql_text, query_context)
        .await?;
        (result, query_id)
    };
    
    handle_query_ok_result(&sql_text, query_id, result, serialization_format)
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
