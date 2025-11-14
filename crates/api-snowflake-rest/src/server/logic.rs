use super::state::AppState;
use crate::models::{
    JsonResponse, LoginRequestData, LoginResponse, LoginResponseData, QueryRequest,
    QueryRequestBody,
};
use crate::server::error::{self as api_snowflake_rest_error, Result};
use crate::server::helpers::handle_query_ok_result;
use executor::models::QueryContext;
use uuid::Uuid;

#[tracing::instrument(
    name = "api_snowflake_rest::logic::login",
    level = "debug",
    skip(state, credentials),
    err,
    ret(level = tracing::Level::TRACE)
)]
pub async fn handle_login_request(
    state: &AppState,
    credentials: LoginRequestData,
) -> Result<LoginResponse> {
    let LoginRequestData {
        login_name,
        password,
        ..
    } = credentials;

    if login_name != *state.config.auth.demo_user || password != *state.config.auth.demo_password {
        return api_snowflake_rest_error::InvalidAuthDataSnafu.fail();
    }

    let session_id = Uuid::new_v4().to_string();
    let _ = state.execution_svc.create_session(&session_id).await?;

    Ok(LoginResponse {
        data: Option::from(LoginResponseData { token: session_id }),
        success: true,
        message: Option::from("successfully executed".to_string()),
    })
}

#[tracing::instrument(
    name = "api_snowflake_rest::logic::query",
    level = "debug",
    skip(state, query_body, client_ip),
    fields(request_id = %query.request_id),
    err,
    ret(level = tracing::Level::TRACE)
)]
pub async fn handle_query_request(
    state: &AppState,
    session_id: &str,
    query: QueryRequest,
    query_body: QueryRequestBody,
    client_ip: Option<String>,
) -> Result<JsonResponse> {
    let QueryRequestBody {
        sql_text,
        async_exec,
    } = query_body;

    let serialization_format = state.config.dbt_serialization_format;
    let mut query_context = QueryContext::default()
        .with_async_query(async_exec)
        .with_request_id(query.request_id);

    if let Some(ip) = client_ip {
        query_context = query_context.with_ip_address(ip);
    }

    if async_exec {
        return api_snowflake_rest_error::NotImplementedSnafu.fail();
    }

    let result = state
        .execution_svc
        .query(session_id, &sql_text, query_context)
        .await?;

    handle_query_ok_result(&sql_text, result, serialization_format)
}
