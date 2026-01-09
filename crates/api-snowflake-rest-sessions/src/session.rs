use crate::error as session_error;
use crate::error::BadAuthTokenSnafu;
use crate::helpers::get_claims_validate_jwt_token;
use axum::extract::FromRequestParts;
use executor::ExecutionAppState;
use executor::SessionMetadata;
use executor::service::ExecutionService;
use http::header::COOKIE;
use http::request::Parts;
use http::{HeaderMap, HeaderName};
use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub const SESSION_ID_COOKIE_NAME: &str = "session_id";

pub const SESSION_EXPIRATION_SECONDS: u64 = 60;

#[derive(Clone)]
pub struct SessionStore {
    pub execution_svc: Arc<dyn ExecutionService>,
}

impl SessionStore {
    pub fn new(execution_svc: Arc<dyn ExecutionService>) -> Self {
        Self { execution_svc }
    }
    pub async fn continuously_delete_expired(&self, period: tokio::time::Duration) {
        let mut interval = tokio::time::interval(period);
        interval.tick().await; // The first tick completes immediately; skip.
        loop {
            interval.tick().await;
            let _ = self.execution_svc.delete_expired_sessions().await;
        }
    }
}

pub trait JwtSecret {
    fn jwt_secret(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizedSession(pub String, pub SessionMetadata);

impl Default for TokenizedSession {
    fn default() -> Self {
        Self(Uuid::new_v4().to_string(), SessionMetadata::default())
    }
}

impl TokenizedSession {
    #[must_use]
    pub fn new(session_id: String) -> Self {
        Self(session_id, SessionMetadata::default())
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: SessionMetadata) -> Self {
        self.1 = metadata;
        self
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn metadata(&self) -> &SessionMetadata {
        &self.1
    }
}

impl<S> FromRequestParts<S> for TokenizedSession
where
    S: Send + Sync + ExecutionAppState + JwtSecret,
{
    type Rejection = session_error::Error;

    #[allow(clippy::unwrap_used)]
    #[tracing::instrument(
        level = "debug",
        skip(req, state),
        fields(session_id, located_at, metadata)
    )]
    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let execution_svc = state.get_execution_svc();

        // Not using Host extractor as for some reason it extracts host without port
        // use axum::RequestPartsExt;
        // use axum::extract::Extension;
        // use crate::layer::Host;
        // let Extension(Host(host)) = req.extract::<Extension<Host>>()
        //     .await
        //     .context(session_error::ExtensionRejectionSnafu)?;
        // tracing::info!("Host '{host}' extracted from TokenizedSession");

        let (session, located_at) = if let Some(token) = extract_token_from_auth(&req.headers) {
            // host is require to check token audience claim
            let host = req.headers.get("host");
            let host = host.and_then(|host| host.to_str().ok());
            let host = host.context(session_error::MissingHostSnafu)?;

            let jwt_secret = state.jwt_secret();
            let jwt_claims = get_claims_validate_jwt_token(&token, host, jwt_secret)
                .context(BadAuthTokenSnafu)?;

            (jwt_claims.session, "auth header")
        } else {
            //This is guaranteed by the `propagate_session_cookie`, so we can unwrap
            let session = req.extensions.get::<Self>().unwrap();
            (session.clone(), "extensions")
        };

        // Record the result as part of the current span.
        tracing::Span::current()
            .record("located_at", located_at)
            .record("metadata", format!("{:?}", session.metadata()))
            .record("session_id", session.session_id());

        Self::get_or_create_session(execution_svc, session).await
    }
}

impl TokenizedSession {
    #[tracing::instrument(
        name = "TokenizedSession::get_or_create_session",
        level = "info",
        skip(execution_svc),
        fields(new_session, sessions_count)
    )]
    async fn get_or_create_session(
        execution_svc: Arc<dyn ExecutionService>,
        session: Self,
    ) -> Result<Self, session_error::Error> {
        let session_id = session.session_id();
        if !execution_svc
            .update_session_expiry(session_id)
            .await
            .context(session_error::ExecutionSnafu)?
        {
            let _ = execution_svc
                .create_session(session_id)
                .await
                .context(session_error::ExecutionSnafu)?;
            tracing::Span::current().record("new_session", true);
        }

        let sessions_count = execution_svc.get_sessions().read().await.len();
        // Record the result as part of the current span.
        tracing::Span::current().record("sessions_count", sessions_count);

        Ok(session)
    }
}

//Snowflake token extraction lives in the api-session crate (used here also),
// so to not create a cyclic dependency exporting it from the api-snowflake-rest crate.
// Where it's used in the `require_auth` layer as part of the session flow and where it was originally from.
#[must_use]
pub fn extract_token_from_auth(headers: &HeaderMap) -> Option<String> {
    //First we check the header
    headers.get("authorization").and_then(|value| {
        value.to_str().ok().and_then(|auth| {
            #[allow(clippy::unwrap_used)]
            let re = Regex::new(
                r#"Snowflake Token="([A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+|[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})""#
            ).unwrap();
            re.captures(auth)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        })
    })
}

#[must_use]
pub fn extract_token_from_cookie(headers: &HeaderMap) -> Option<String> {
    let cookies = cookies_from_header(headers, COOKIE);
    cookies
        .get(SESSION_ID_COOKIE_NAME)
        .map(|str_ref| (*str_ref).to_string())
}

#[allow(clippy::explicit_iter_loop)]
pub fn cookies_from_header(headers: &HeaderMap, header_name: HeaderName) -> HashMap<&str, &str> {
    let mut cookies_map = HashMap::new();

    let cookies = headers.get_all(header_name);

    for value in cookies.iter() {
        if let Ok(cookie_str) = value.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().split('=').collect();
                if parts.len() > 1 {
                    cookies_map.insert(parts[0], parts[1]);
                }
            }
        }
    }
    cookies_map
}

#[cfg(test)]
mod tests {
    use crate::session::SessionStore;
    use executor::models::QueryContext;
    use executor::service::ExecutionService;
    use executor::service::make_test_execution_svc;
    use executor::session::to_unix;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use time::OffsetDateTime;
    use tokio::time::sleep;

    #[tokio::test]
    #[allow(clippy::expect_used, clippy::too_many_lines)]
    async fn test_expiration() {
        let execution_svc = make_test_execution_svc().await;

        let df_session_id = "fasfsafsfasafsass".to_string();
        let user_session = execution_svc
            .create_session(&df_session_id)
            .await
            .expect("Failed to create a session");

        user_session
            .expiry
            .store(to_unix(OffsetDateTime::now_utc()), Ordering::Relaxed);

        let session_store = SessionStore::new(execution_svc.clone());

        tokio::task::spawn({
            let session_store = session_store.clone();
            async move {
                session_store
                    .continuously_delete_expired(Duration::from_secs(5))
                    .await;
            }
        });

        let () = sleep(Duration::from_secs(7)).await;
        execution_svc
            .query(&df_session_id, "SELECT 1", QueryContext::default())
            .await
            .expect_err("Failed to execute query (session deleted)");
    }
}
