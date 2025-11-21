use super::{error, state::AppState};
use crate::server::error::{BadAuthTokenSnafu, NoJwtSecretSnafu};
use api_snowflake_rest_sessions::helpers::{
    ensure_jwt_secret_is_valid, get_claims_validate_jwt_token,
};
use api_snowflake_rest_sessions::layer::Host;
use api_snowflake_rest_sessions::session::extract_token_from_auth;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::IntoResponse;
use snafu::{OptionExt, ResultExt};

#[allow(clippy::unwrap_used)]
#[tracing::instrument(
    name = "api_snowflake_rest::layer::require_auth",
    level = "trace",
    skip(state, req, next),
    fields(request_headers = format!("{:#?}", req.headers()), response_headers, session_id),
    err,
)]
pub async fn require_auth(
    State(state): State<AppState>,
    Host(host): Host,
    req: Request,
    next: Next,
) -> error::Result<impl IntoResponse> {
    // no demo user -> no auth required
    if state.config.auth.demo_user.is_empty() || state.config.auth.demo_password.is_empty() {
        return Ok(next.run(req).await);
    }

    let Some(token) = extract_token_from_auth(req.headers()) else {
        return error::MissingAuthTokenSnafu.fail()?;
    };

    let jwt_secret =
        ensure_jwt_secret_is_valid(&state.config.auth.jwt_secret).context(NoJwtSecretSnafu)?;

    tracing::info!("Host '{host}' for token validation");

    let jwt_claims =
        get_claims_validate_jwt_token(&token, &host, &jwt_secret).context(BadAuthTokenSnafu)?;

    // Record the result as part of the current span.
    tracing::Span::current().record("session_id", jwt_claims.session_id.as_str());

    let response = next.run(req).await;

    // Record the result as part of the current span.
    tracing::Span::current().record("response_headers", format!("{:#?}", response.headers()));

    Ok(response)
}
