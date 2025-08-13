#![allow(clippy::needless_for_each)]
use super::error::AuthErrorResponse;
use super::error::CreateJwtSnafu;
use crate::auth::error::{self as auth_error, BadRefreshTokenSnafu, TokenErrorKind};
use crate::auth::models::{AccountResponse, RefreshTokenResponse};
use crate::auth::models::{AuthResponse, Claims, LoginPayload};
use crate::error::Result;
use crate::state::AppState;

use api_sessions::session::SESSION_ID_COOKIE_NAME;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::offset::Local;
use http::HeaderMap;
use http::header::SET_COOKIE;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::Serialize;
use snafu::ResultExt;
use std::collections::HashMap;
use time::Duration;
use tower_sessions::cookie::{Cookie, SameSite};
use tracing;
use utoipa::OpenApi;

pub const REFRESH_TOKEN_EXPIRATION_HOURS: u32 = 24 * 7;
pub const ACCESS_TOKEN_EXPIRATION_SECONDS: u32 = 15 * 60;
pub const REFRESH_COOKIE_NAME: &str = "refresh_token";

#[allow(clippy::explicit_iter_loop)]
pub fn cookies_from_header(headers: &HeaderMap) -> HashMap<&str, &str> {
    let mut cookies_map = HashMap::new();

    let cookies = headers.get_all(http::header::COOKIE);

    for value in cookies.iter() {
        if let Ok(cookie_str) = value.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().split('=').collect();
                cookies_map.insert(parts[0], parts[1]);
            }
        }
    }
    cookies_map
}

#[must_use]
pub fn jwt_claims(username: &str, audience: &str, expiration: Duration) -> Claims {
    let now = Local::now();
    let iat = now.timestamp();
    let exp = now.timestamp() + expiration.whole_seconds();

    Claims {
        sub: username.to_string(),
        aud: audience.to_string(),
        iat,
        exp,
    }
}

#[must_use]
fn access_token_claims(username: &str, audience: &str) -> Claims {
    jwt_claims(
        username,
        audience,
        Duration::seconds(ACCESS_TOKEN_EXPIRATION_SECONDS.into()),
    )
}

#[must_use]
fn refresh_token_claims(username: &str, audience: &str) -> Claims {
    jwt_claims(
        username,
        audience,
        Duration::hours(REFRESH_TOKEN_EXPIRATION_HOURS.into()),
    )
}

pub fn get_claims_validate_jwt_token(
    token: &str,
    audience: &str,
    jwt_secret: &str,
) -> std::result::Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.leeway = 5;
    validation.set_audience(&[audience]);
    validation.set_required_spec_claims(&["exp", "aud"]);

    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;

    Ok(decoded.claims)
}

pub fn create_jwt<T>(
    claims: &T,
    jwt_secret: &str,
) -> std::result::Result<String, jsonwebtoken::errors::Error>
where
    T: Serialize,
{
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

fn ensure_jwt_secret_is_valid(jwt_secret: &str) -> Result<()> {
    if jwt_secret.is_empty() {
        return auth_error::NoJwtSecretSnafu.fail()?;
    }
    Ok(())
}

fn set_cookies(headers: &mut HeaderMap, name: &str, token: &str) -> Result<()> {
    headers
        .try_append(
            SET_COOKIE,
            Cookie::build((name, token))
                .http_only(true)
                .secure(true)
                .same_site(SameSite::Strict)
                .path("/")
                .to_string()
                .parse()
                .context(auth_error::ResponseHeaderSnafu)?,
        )
        .context(auth_error::SetCookieSnafu)?;

    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        login,
        refresh_access_token,
        logout,
        account,
    ),
    components(
        schemas(
            LoginPayload,
            AuthResponse,
            RefreshTokenResponse,
            AuthErrorResponse,
            TokenErrorKind,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints")
    )
)]
pub struct ApiDoc;

#[utoipa::path(
    post,
    path = "/ui/auth/login",
    operation_id = "login",
    tags = ["auth"],
    request_body = LoginPayload,
    responses(
        (status = 200, description = "Successful Response", body = AuthResponse),
        (status = 401,
         description = "Unauthorized", 
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = AuthErrorResponse),
        (status = 500, description = "Internal server error", body = AuthErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::login", level = "info", skip_all, err)]
pub async fn login(
    State(state): State<AppState>,
    Json(LoginPayload { username, password }): Json<LoginPayload>,
) -> Result<impl IntoResponse> {
    if username != *state.auth_config.demo_user() || password != *state.auth_config.demo_password()
    {
        return auth_error::LoginSnafu.fail()?;
    }

    let audience = state.config.host.clone();
    let jwt_secret = state.auth_config.jwt_secret();

    ensure_jwt_secret_is_valid(jwt_secret)?;

    let access_token_claims = access_token_claims(&username, &audience);

    let access_token = create_jwt(&access_token_claims, jwt_secret).context(CreateJwtSnafu)?;

    let refresh_token_claims = refresh_token_claims(&username, &audience);

    let refresh_token = create_jwt(&refresh_token_claims, jwt_secret).context(CreateJwtSnafu)?;

    let mut headers = HeaderMap::new();
    set_cookies(&mut headers, REFRESH_COOKIE_NAME, &refresh_token)?;

    Ok((
        headers,
        Json(AuthResponse::new(
            access_token,
            ACCESS_TOKEN_EXPIRATION_SECONDS,
        )),
    ))
}

#[utoipa::path(
    post,
    path = "/ui/auth/refresh",
    operation_id = "refreshAuthToken",
    tags = ["auth"],
    responses(
        (status = 200, description = "Successful Response", body = AuthResponse),
        (status = 401,
            description = "Unauthorized", 
            headers(
               ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
            ),
            body = AuthErrorResponse),
        (status = 500, description = "Internal server error", body = AuthErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::refresh_access_token", level = "info", skip_all, err)]
pub async fn refresh_access_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let jwt_secret = state.auth_config.jwt_secret();
    ensure_jwt_secret_is_valid(jwt_secret)?;

    let cookies_map = cookies_from_header(&headers);
    match cookies_map.get(REFRESH_COOKIE_NAME) {
        None => auth_error::NoRefreshTokenCookieSnafu.fail()?,
        Some(refresh_token) => {
            let refresh_claims =
                get_claims_validate_jwt_token(refresh_token, &state.config.host, jwt_secret)
                    .context(BadRefreshTokenSnafu)?;

            let access_claims = access_token_claims(&refresh_claims.sub, &state.config.host);

            let access_token = create_jwt(&access_claims, jwt_secret).context(CreateJwtSnafu)?;

            let mut headers = HeaderMap::new();
            set_cookies(&mut headers, REFRESH_COOKIE_NAME, refresh_token)?;

            Ok((
                headers,
                Json(AuthResponse::new(
                    access_token,
                    ACCESS_TOKEN_EXPIRATION_SECONDS,
                )),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/ui/auth/logout",
    operation_id = "logout",
    tags = ["auth"],
    responses(
        (status = 200, description = "Successful Response"),
        (status = 401,
            description = "Unauthorized", 
            headers(
               ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
            ),
            body = AuthErrorResponse),
        (status = 500, description = "Internal server error", body = AuthErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::logout", level = "info", skip_all, err)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let jwt_secret = state.auth_config.jwt_secret();
    ensure_jwt_secret_is_valid(jwt_secret)?;

    let cookies_map = cookies_from_header(&headers);

    if let Some(refresh_token) = cookies_map.get(REFRESH_COOKIE_NAME) {
        let audience = state.config.host.clone();

        let _ = get_claims_validate_jwt_token(refresh_token, &audience, jwt_secret)
            .context(BadRefreshTokenSnafu)?;
        // logout doesn't return unauthorized
    }

    // unset refresh_token, access_token cookies

    let mut headers = HeaderMap::new();
    set_cookies(&mut headers, REFRESH_COOKIE_NAME, "")?;
    set_cookies(&mut headers, SESSION_ID_COOKIE_NAME, "")?;

    Ok((headers, ()))
}

#[utoipa::path(
    get,
    path = "/ui/auth/account",
    operation_id = "getAccount",
    tags = ["account"],
    responses(
        (status = 200, description = "Successful Response", body = AccountResponse),
        (status = 401,
            description = "Unauthorized", 
            headers(
               ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
            ),
            body = AuthErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::account", level = "info", skip_all, err)]
pub async fn account(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    // Simplest account info, also no auth checks.
    // TODO: Move it to proper place when working with real account
    // Check authentication

    let auth = headers.get(http::header::AUTHORIZATION);
    if auth.is_some() {
        Ok((
            headers,
            Json(AccountResponse {
                username: state.auth_config.demo_user().to_string(),
            }),
        ))
    } else {
        auth_error::NoAuthHeaderSnafu.fail()?
    }
}
