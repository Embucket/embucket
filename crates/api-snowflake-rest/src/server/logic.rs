use super::state::AppState;
use crate::models::{
    JsonResponse, LoginRequestData, LoginResponse, LoginResponseData, QueryRequest,
    QueryRequestBody,
};
use crate::server::error::{
    self as api_snowflake_rest_error, CreateJwtSnafu, NoJwtSecretSnafu, Result,
};
use crate::server::helpers::handle_query_ok_result;
use api_snowflake_rest_sessions::helpers::{create_jwt, ensure_jwt_secret_is_valid, jwt_claims};
use executor::models::QueryContext;
use snafu::{OptionExt, ResultExt};
use time::Duration;

pub const JWT_TOKEN_EXPIRATION_SECONDS: u32 = 24 * 60 * 60;

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

    let jwt_secret = &*state.config.auth.jwt_secret;
    let _ = ensure_jwt_secret_is_valid(jwt_secret).context(NoJwtSecretSnafu)?;

    let jwt_claims = jwt_claims(
        &login_name,
        Duration::seconds(JWT_TOKEN_EXPIRATION_SECONDS.into()),
    );

    let session_id = jwt_claims.session_id.clone();
    let _ = state.execution_svc.create_session(&session_id).await?;

    let jwt_token = create_jwt(&jwt_claims, jwt_secret).context(CreateJwtSnafu)?;

    Ok(LoginResponse {
        data: Option::from(LoginResponseData { token: jwt_token }),
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
    let async_exec = async_exec.unwrap_or(false);

    let serialization_format = state.config.dbt_serialization_format;
    let mut query_context = QueryContext::default().with_request_id(query.request_id);

    if let Some(ip) = client_ip {
        query_context = query_context.with_ip_address(ip);
    }

    if async_exec {
        return api_snowflake_rest_error::NotImplementedSnafu.fail();
    }

    let query_uuid = query_context.query_id.as_uuid();
    let result = state
        .execution_svc
        .query(session_id, &sql_text, query_context)
        .await?;

    handle_query_ok_result(&sql_text, query_uuid, result, serialization_format)
}
