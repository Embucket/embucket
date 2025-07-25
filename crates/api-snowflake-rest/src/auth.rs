use crate::handlers::login;
use crate::state::AppState;
use axum::Router;
use axum::routing::post;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/session/v1/login-request", post(login))
}
