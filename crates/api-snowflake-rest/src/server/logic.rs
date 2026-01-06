use super::state::AppState;
use crate::models::{
    JsonResponse, LoginRequestData, LoginRequestQueryParams, LoginResponse, LoginResponseData,
    QueryRequest, QueryRequestBody,
};
use crate::server::error::{
    self as api_snowflake_rest_error, CreateJwtSnafu, NoJwtSecretSnafu, Result, SetVariableSnafu,
};
use crate::server::helpers::handle_query_ok_result;
use api_snowflake_rest_sessions::helpers::{create_jwt, ensure_jwt_secret_is_valid, jwt_claims};
use executor::RunningQueryId;
use executor::models::QueryContext;
use snafu::{OptionExt, ResultExt};
use time::Duration;

pub const JWT_TOKEN_EXPIRATION_SECONDS: u32 = 3 * 24 * 60 * 60;

#[tracing::instrument(
    name = "api_snowflake_rest::logic::login",
    level = "debug",
    skip(state, credentials),
    err,
    ret(level = tracing::Level::TRACE)
)]
pub async fn handle_login_request(
    state: &AppState,
    host: String,
    credentials: LoginRequestData,
    params: LoginRequestQueryParams,
    client_ip: Option<String>,
) -> Result<LoginResponse> {
    let LoginRequestData {
        login_name,
        password,
        ..
    } = credentials;

    if login_name != *state.config.auth.demo_user || password != *state.config.auth.demo_password {
        return api_snowflake_rest_error::InvalidAuthDataSnafu.fail();
    }

    // host is required to check token audience claim
    let jwt_secret = &*state.config.auth.jwt_secret;
    let _ = ensure_jwt_secret_is_valid(jwt_secret).context(NoJwtSecretSnafu)?;

    let jwt_claims = jwt_claims(
        &login_name,
        &host,
        Duration::seconds(JWT_TOKEN_EXPIRATION_SECONDS.into()),
    );

    tracing::info!("Host '{host}' for token creation");

    let session_id = jwt_claims.session_id.clone();
    let session = state.execution_svc.create_session(&session_id).await?;

    let jwt_token = create_jwt(&jwt_claims, jwt_secret).context(CreateJwtSnafu)?;

    // set database, schema when provided
    if let Some(db) = params.database_name {
        session.set_database(&db).await.context(SetVariableSnafu {
            variable: "database",
        })?;
    }
    if let Some(schema) = params.schema_name {
        session
            .set_schema(&schema)
            .await
            .context(SetVariableSnafu { variable: "schema" })?;
    }
    if let Some(warehouse) = params.warehouse {
        session
            .set_warehouse(&warehouse)
            .await
            .context(SetVariableSnafu {
                variable: "warehouse",
            })?;
    }

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
    if async_exec {
        return api_snowflake_rest_error::NotImplementedSnafu.fail();
    }

    let serialization_format = state.config.dbt_serialization_format;
    let mut query_context = QueryContext::default().with_request_id(query.request_id);

    if let Some(ip) = client_ip {
        query_context = query_context.with_ip_address(ip);
    }

    // find running query by request_id
    let query_id_res = state
        .execution_svc
        .locate_query_id(RunningQueryId::ByRequestId(
            query.request_id,
            sql_text.clone(),
        ));

    // if retry-disable feature is enabled we ignory retries regardless of query_id is located or not
    #[cfg(feature = "retry-disable")]
    if query.retry_count.unwrap_or_default() > 0 {
        return api_snowflake_rest_error::RetryDisabledSnafu.fail();
    }

    let (result, query_id) = if query.retry_count.unwrap_or_default() > 0
        && let Ok(query_id) = query_id_res
    {
        let result = state.execution_svc.wait(query_id).await?;
        (result, query_id)
    } else {
        let query_id = query_context.query_id;
        let result = state
            .execution_svc
            .query(session_id, &sql_text, query_context)
            .await?;
        (result, query_id)
    };

    handle_query_ok_result(&sql_text, query_id, result, serialization_format)
}
