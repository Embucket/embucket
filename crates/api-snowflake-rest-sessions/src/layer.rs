use crate::error as session_error;
use crate::error::{Error, Result};
use crate::session::{
    DFSessionId, SESSION_ID_COOKIE_NAME, SessionStore, extract_token_from_cookie,
};
use axum::extract::{FromRequestParts, Request, State};
use axum::http::{HeaderMap, HeaderName, request::Parts};
use axum::middleware::Next;
use axum::response::IntoResponse;
use http::header::SET_COOKIE;
use snafu::ResultExt;
use tower_sessions::cookie::{Cookie, SameSite};

#[derive(Debug, Clone)]
pub struct Host(pub String);

impl<S> FromRequestParts<S> for Host
where
    S: Send + Sync,
{
    type Rejection = Error;

    #[allow(clippy::unwrap_used)]
    async fn from_request_parts(
        req: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let headers = HeaderMap::from_request_parts(req, state)
            .await
            .map_err(|err| match err {})
            .unwrap(); // unwrap on Infallibe error is safe
        let host = headers.get("host");
        let host = host.and_then(|host| host.to_str().ok());
        if let Some(host) = host {
            Ok(Self(host.to_string()))
        } else {
            session_error::MissingHostSnafu.fail()
        }
    }
}

#[allow(clippy::unwrap_used, clippy::cognitive_complexity)]
pub async fn propagate_session_cookie(
    State(state): State<SessionStore>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse> {
    if let Some(token) = extract_token_from_cookie(req.headers()) {
        tracing::debug!(session_id = %token, "Found DF session_id in cookie header");

        //session_id is expired and deleted
        if !state.execution_svc.session_exists(&token).await {
            tracing::debug!("This DF session_id is expired or deleted.");

            let session_id = uuid::Uuid::new_v4().to_string();
            //Propagate new session_id to the extractor
            req.extensions_mut().insert(DFSessionId(session_id.clone()));
            let mut res = next.run(req).await;
            set_headers_in_flight(
                res.headers_mut(),
                SET_COOKIE,
                SESSION_ID_COOKIE_NAME,
                session_id.as_str(),
            )?;
            return Ok(res);
        }
        tracing::debug!("This DF session_id is not expired or deleted.");
        //Propagate in-use (valid) session_id to the extractor
        req.extensions_mut().insert(DFSessionId(token));
    } else {
        let session_id = uuid::Uuid::new_v4().to_string();
        tracing::debug!(session_id = %session_id, "Created new DF session_id");
        //Propagate new session_id to the extractor
        req.extensions_mut().insert(DFSessionId(session_id.clone()));
        let mut res = next.run(req).await;
        set_headers_in_flight(
            res.headers_mut(),
            SET_COOKIE,
            SESSION_ID_COOKIE_NAME,
            session_id.as_str(),
        )?;
        return Ok(res);
    }

    Ok(next.run(req).await)
}

fn set_headers_in_flight(
    headers: &mut HeaderMap,
    header_name: HeaderName,
    name: &str,
    token: &str,
) -> Result<()> {
    headers
        .try_insert(
            header_name,
            Cookie::build((name, token))
                .http_only(true)
                .secure(true)
                .same_site(SameSite::Strict)
                .path("/")
                .to_string()
                .parse()
                .context(session_error::ResponseHeaderSnafu)?,
        )
        .context(session_error::SetCookieSnafu)?;

    Ok(())
}
